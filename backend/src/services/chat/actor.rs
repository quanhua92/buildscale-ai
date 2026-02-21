use crate::models::agent_session::AgentType;
use crate::models::chat::{ChatMessageMetadata, ChatMessageRole, NewChatMessage, DEFAULT_CHAT_MODEL};
use crate::models::sse::SseEvent;
use crate::providers::Agent;
use crate::queries;
use crate::services::agent_sessions;
use crate::services::chat::registry::{AgentCommand, AgentHandle, AgentRegistry};
use crate::services::chat::rig_engine::RigService;
use crate::services::chat::ChatService;
use crate::services::chat::state_machine::{ActorEvent, ActorState, StateMachine, StateAction};
use crate::services::chat::states::{SharedActorState, StateContext, StateHandlerRegistry};
use crate::services::storage::FileStorageService;
use crate::DbPool;
use crate::error::Result;
use futures::StreamExt;
use rig::streaming::StreamingChat;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, instrument};
use uuid::Uuid;

/// Maximum number of retries for transient AI engine errors
const MAX_AI_RETRIES: u32 = 3;
/// Initial backoff duration in milliseconds for retries
const RETRY_BACKOFF_MS: u64 = 1000;
/// Stream read timeout in seconds - if no data received from API within this time, consider it stalled
const STREAM_READ_TIMEOUT_SECS: u64 = 120;

/// Consolidated state for ChatActor to reduce lock contention
/// All state that was previously in separate Arc<Mutex<>> fields is now grouped logically
struct ChatActorState {
    /// Tool Tracking (always accessed in pairs)
    tool_tracking: ToolTracking,
    /// Interaction Lifecycle (independent access)
    interaction: InteractionState,
    /// Current reasoning session tracking (for audit trail)
    current_reasoning_id: Option<String>,
    /// Buffer for reasoning chunks (aggregated before DB persistence)
    reasoning_buffer: Vec<String>,
}

/// Current tool execution tracking
/// These fields are always read/written together
struct ToolTracking {
    /// Track current tool name for logging when ToolResult arrives
    current_tool_name: Option<String>,
    /// Track current tool arguments for logging when ToolResult arrives
    current_tool_args: Option<serde_json::Value>,
}

/// Interaction lifecycle management
/// These fields manage the current interaction's lifecycle
struct InteractionState {
    /// Cancellation token for the current interaction
    current_cancellation_token: Option<CancellationToken>,
    /// Track current model for cancellation metadata
    current_model: Option<String>,
    /// Current task description for session tracking
    current_task: Option<String>,
    /// Flag to track if the actor is actively processing an interaction
    /// Used to prevent inactivity timeout during long-running tasks
    is_actively_processing: bool,
}

impl ChatActorState {
    fn ensure_reasoning_id(&mut self) -> String {
        self.current_reasoning_id
            .get_or_insert_with(|| Uuid::now_v7().to_string())
            .clone()
    }
}

impl Default for ChatActorState {
    fn default() -> Self {
        Self {
            tool_tracking: ToolTracking {
                current_tool_name: None,
                current_tool_args: None,
            },
            interaction: InteractionState {
                current_cancellation_token: None,
                current_model: None,
                current_task: None,
                is_actively_processing: false,
            },
            current_reasoning_id: None,
            reasoning_buffer: Vec::new(),
        }
    }
}

pub struct ChatActor {
    chat_id: Uuid,
    workspace_id: Uuid,
    user_id: Uuid,
    pool: DbPool,
    rig_service: Arc<RigService>,
    storage: Arc<FileStorageService>,
    registry: Arc<AgentRegistry>,
    command_rx: mpsc::Receiver<AgentCommand>,
    event_tx: broadcast::Sender<SseEvent>,
    default_persona: String,
    default_context_token_limit: usize,
    inactivity_timeout: std::time::Duration,
    /// Agent session ID for tracking this actor in the database
    session_id: Option<Uuid>,
    /// Handle for the heartbeat task
    heartbeat_handle: Option<JoinHandle<()>>,
    /// Consolidated state - single lock for all actor state
    /// Reduces lock contention and eliminates deadlock risk
    state: Arc<Mutex<ChatActorState>>,
    /// Shared state for state handlers (NEW - simplified, no agent cache)
    shared_state: Arc<Mutex<SharedActorState>>,
    /// State machine for managing actor lifecycle (NEW)
    state_machine: StateMachine,
    /// State handlers for state-specific behavior (NEW)
    state_handlers: StateHandlerRegistry,
}

pub struct ChatActorArgs {
    pub chat_id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub pool: DbPool,
    pub rig_service: Arc<RigService>,
    pub storage: Arc<FileStorageService>,
    pub registry: Arc<AgentRegistry>,
    pub default_persona: String,
    pub default_context_token_limit: usize,
    pub event_tx: broadcast::Sender<SseEvent>,
    pub inactivity_timeout: std::time::Duration,
}

impl ChatActor {
    pub fn spawn(
        args: ChatActorArgs,
    ) -> AgentHandle {
        Self::spawn_with_args(args)
    }

    fn spawn_with_args(args: ChatActorArgs) -> AgentHandle {
        let (command_tx, command_rx) = mpsc::channel(32);
        let event_tx = args.event_tx.clone();

        // Initialize state machine in Idle state
        let state_machine = StateMachine::new(ActorState::Idle);

        // Initialize state handlers registry
        let state_handlers = StateHandlerRegistry::new();

        let actor = Self {
            chat_id: args.chat_id,
            workspace_id: args.workspace_id,
            user_id: args.user_id,
            pool: args.pool.clone(),
            rig_service: args.rig_service,
            storage: args.storage,
            registry: args.registry,
            command_rx,
            event_tx: args.event_tx.clone(),
            default_persona: args.default_persona,
            default_context_token_limit: args.default_context_token_limit,
            inactivity_timeout: args.inactivity_timeout,
            session_id: None,
            heartbeat_handle: None,
            state: Arc::new(Mutex::new(ChatActorState::default())),
            shared_state: Arc::new(Mutex::new(SharedActorState::default())),
            state_machine,
            state_handlers,
        };

        tokio::spawn(async move {
            actor.run().await;
        });

        AgentHandle {
            command_tx,
            event_tx,
        }
    }

    async fn run(mut self) {
        tracing::info!(
            chat_id = %self.chat_id,
            workspace_id = %self.workspace_id,
            user_id = %self.user_id,
            inactivity_timeout_secs = self.inactivity_timeout.as_secs(),
            "[ChatActor] STARTED - Agent lifecycle beginning"
        );

        // Log when entering the main loop
        tracing::debug!(
            chat_id = %self.chat_id,
            "[ChatActor] Entering main event loop"
        );

        // Create agent session in database
        let session_result = self.create_session().await;

        // Treat session creation failure as fatal error - shut down actor
        let session_id = match session_result {
            Ok(id) => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    session_id = %id,
                    "[ChatActor] Session created, starting heartbeat task"
                );
                id
            }
            Err(e) => {
                tracing::error!(
                    chat_id = %self.chat_id,
                    error = ?e,
                    "[ChatActor] Failed to create session - shutting down actor"
                );
                return;
            }
        };

        self.session_id = Some(session_id);
        self.heartbeat_handle = Some(self.start_heartbeat_task(session_id));

        // Log initial state entry (actor starts in Idle state)
        self.log_state_transition(
            ActorState::Idle,
            ActorState::Idle,
            "Actor started - entering Idle state"
        );

        // Periodic heartbeat ping (every 10 seconds)
        let mut heartbeat_interval = tokio::time::interval(std::time::Duration::from_secs(10));

        // Inactivity timeout (shutdown after no commands)
        let inactivity_timeout_duration = self.inactivity_timeout;
        let inactivity_timeout = tokio::time::sleep(inactivity_timeout_duration);
        tokio::pin!(inactivity_timeout);

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    let _ = self.event_tx.send(SseEvent::Ping);
                }
                _ = &mut inactivity_timeout => {
                    // Only timeout if NOT actively processing
                    let is_actively_processing = self.state.lock().await.interaction.is_actively_processing;

                    if !is_actively_processing {
                        tracing::info!(
                            chat_id = %self.chat_id,
                            reason = "inactivity_timeout",
                            "[ChatActor] SHUTTING DOWN - No commands received while idle"
                        );

                        // State transition: → Completed (terminal state)
                        let _ = self.transition_state(
                            ActorEvent::InactivityTimeout,
                            "Inactivity timeout reached"
                        ).await;

                        // Update session status to completed in database
                        if let Some(session_id) = self.session_id {
                            let _ = self.update_session_status(
                                session_id,
                                crate::models::agent_session::SessionStatus::Completed,
                                None,
                            ).await;
                            tracing::debug!(
                                chat_id = %self.chat_id,
                                session_id = %session_id,
                                "[ChatActor] Session marked as completed due to inactivity timeout"
                            );
                        }

                        break;
                    }

                    // Reset timeout if actively processing - the actor is busy
                    tracing::debug!(
                        chat_id = %self.chat_id,
                        "[ChatActor] Inactivity timeout fired but actor is actively processing, resetting timeout"
                    );
                    inactivity_timeout.as_mut().reset(tokio::time::Instant::now() + inactivity_timeout_duration);
                }
                command = self.command_rx.recv() => {
                    if let Some(cmd) = command {
                        // Reset inactivity timeout on any command
                        inactivity_timeout.as_mut().reset(tokio::time::Instant::now() + inactivity_timeout_duration);

                        // Try to process the command through state handlers first
                        // If the handler returns true, the command was fully handled
                        // If the handler returns false or errors, fall back to legacy path
                        let handled_by_state_handler = match self.process_command_via_state_handler(&cmd).await {
                            Ok(handled) => handled,
                            Err(e) => {
                                warn!(
                                    error = ?e,
                                    "[ChatActor] State handler processing failed, falling back to legacy path"
                                );
                                false
                            }
                        };

                        // If state handler handled it, continue to next command
                        if handled_by_state_handler {
                            // Check if we should break (terminal state)
                            if self.current_state().is_terminal() {
                                break;
                            }
                            continue;
                        }

                        // Fall back to legacy command processing
                        match cmd {
                            // Note: ProcessInteraction is now fully handled by the state machine
                            // This arm should never be reached since process_command_via_state_handler
                            // handles it and returns true (command was handled)
                            AgentCommand::ProcessInteraction { .. } => {
                                unreachable!("ProcessInteraction should be handled by state machine")
                            }
                            AgentCommand::Ping => {
                                tracing::debug!("ChatActor received ping for chat {}", self.chat_id);
                            }
                            // Note: Pause, Cancel, and Shutdown are now fully handled by state handlers
                            // The following arms should never be reached since process_command_via_state_handler
                            // handles these commands before we get to this legacy match.
                            AgentCommand::Pause { .. } => {
                                unreachable!("Pause should be handled by state handlers")
                            }
                            AgentCommand::Cancel { .. } => {
                                unreachable!("Cancel should be handled by state handlers")
                            }
                            AgentCommand::Shutdown => {
                                unreachable!("Shutdown should be handled by state handlers")
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        // Cleanup: stop heartbeat and mark session as completed
        let heartbeat_handle = self.heartbeat_handle.take();
        let session_id = self.session_id.take();

        tracing::info!(
            chat_id = %self.chat_id,
            session_id = ?session_id,
            "[ChatActor] ACTOR EXITING - cleanup starting"
        );

        if let Some(handle) = heartbeat_handle {
            handle.abort();
        }

        // Note: We intentionally do NOT mark the session as completed here.
        // The session record should persist in its last known state (e.g., idle, running).
        // - User-cancelled sessions: handler/service owns final DB state
        // - Inactivity timeout: session is NOT "completed", just inactive
        // - Normal shutdown: session stays in its last active state
        // Stale sessions will be handled by the cleanup worker if not re-activated.
        tracing::info!(
            chat_id = %self.chat_id,
            session_id = ?session_id,
            "[ChatActor] EXITED - Actor lifecycle complete"
        );
    }

    async fn process_interaction(&self, user_id: Uuid) -> crate::error::Result<()> {
        tracing::info!(
            chat_id = %self.chat_id,
            user_id = %user_id,
            "[ChatActor] ProcessInteraction STARTED"
        );

        // Log SSE receiver count
        tracing::debug!(
            chat_id = %self.chat_id,
            receivers = self.event_tx.receiver_count(),
            "[ChatActor] Current SSE receiver count"
        );

        // Create a new cancellation token for this interaction
        let cancellation_token = CancellationToken::new();
        self.state.lock().await.interaction.current_cancellation_token = Some(cancellation_token.clone());

        let mut conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

        // 1. Build structured context with persona, history, and attachments
        // Exclude last message (user's prompt) from history since we're responding to it
        let context = ChatService::build_context(
            &mut conn,
            &self.storage,
            self.workspace_id,
            self.chat_id,
            &self.default_persona,
            self.default_context_token_limit,
            true, // exclude_last_message for AI context
        ).await?;
        tracing::debug!(
            chat_id = %self.chat_id,
            persona_len = context.persona.len(),
            history_count = context.history.messages.len(),
            attachment_count = context.attachment_manager.map.len(),
            "Built structured context"
        );

        // 2. Get current message (the prompt)
        let messages =
            queries::chat::get_messages_by_file_id(&mut conn, self.workspace_id, self.chat_id)
                .await?;

        let last_message = messages
            .last()
            .ok_or_else(|| crate::error::Error::Internal("No messages found".into()))?;

        // Set current task for session tracking (truncate if too long)
        let task_preview = crate::utils::safe_preview(&last_message.content, 100);
        self.state.lock().await.interaction.current_task = Some(task_preview.clone());

        tracing::debug!(
            chat_id = %self.chat_id,
            task = %task_preview,
            "[ChatActor] Set current task from user message"
        );

        // Update session with current task
        if let Some(session_id) = self.session_id {
            if let Err(e) = agent_sessions::update_session_task(&mut conn, session_id, Some(task_preview.clone()), self.user_id).await {
                tracing::warn!(
                    session_id = %session_id,
                    error = %e,
                    "[ChatActor] Failed to update session task"
                );
            } else {
                tracing::debug!(
                    session_id = %session_id,
                    task = %task_preview,
                    "[ChatActor] Updated session current_task"
                );
            }
        }

        // 3. Convert history to Rig format with cache-optimized attachment interleaving
        // Attachments are now interleaved chronologically with messages for better caching
        let history = self
            .rig_service
            .convert_history_with_attachments(&context.history.messages, Some(&context.attachment_manager));

        // 4. Build prompt - just the user's message (attachments are now in history)
        let prompt = last_message.content.clone();

        // 6. Hydrate session model
        let file = queries::files::get_file_by_id(&mut conn, self.chat_id).await?;

        let agent_config = if let Some(_version_id) = file.latest_version_id {
            let version = queries::files::get_latest_version(&mut conn, self.chat_id).await?;
            serde_json::from_value(version.app_data).map_err(crate::error::Error::Json)?
        } else {
            tracing::warn!(
                "Chat file {} has no version, using default agent_config.",
                self.chat_id
            );
            crate::models::chat::AgentConfig {
                agent_id: None,
                model: DEFAULT_CHAT_MODEL.to_string(),
                temperature: 0.7,
                persona_override: Some(context.persona),
                previous_response_id: None,
                mode: "plan".to_string(),
                plan_file: None,
            }
        };

        let session = crate::models::chat::ChatSession {
            file_id: self.chat_id,
            agent_config,
            messages: messages.clone(),
        };

        // Store current model for potential cancellation
        self.state.lock().await.interaction.current_model = Some(session.agent_config.model.clone());

        // 7. Load AI config for reasoning settings
        let ai_config = crate::config::Config::load()?.ai;

        // 8. Get or create cached Rig Agent
        tracing::info!(
            chat_id = %self.chat_id,
            user_id = %user_id,
            model = %session.agent_config.model,
            mode = %session.agent_config.mode,
            "Getting or creating agent"
        );
        let agent = self.get_or_create_agent(user_id, &session, &ai_config).await?;
        tracing::info!(
            chat_id = %self.chat_id,
            "Agent created/retrieved successfully"
        );

        // 9. Stream from Rig with persona, history, and attachments in prompt
        tracing::info!(
            chat_id = %self.chat_id,
            prompt_len = prompt.len(),
            history_len = history.len(),
            "Starting agent.stream_chat"
        );

        // Register cancellation token so STOP can cancel even if actor exits
        self.registry.register_cancellation(self.chat_id, cancellation_token.clone()).await;

        let mut item_count = 0usize;

        // Process stream with retry logic for transient errors
        let mut retry_count = 0u32;
        let full_response = loop {
            // Check for cancellation before attempting
            if cancellation_token.is_cancelled() {
                tracing::info!("[ChatActor] Cancelled before streaming for chat {}", self.chat_id);
                return Err(crate::error::Error::Internal("Chat cancelled by user".to_string()));
            }

            let result = match &agent {
                Agent::OpenAI(openai_agent) => {
                    tracing::info!(
                        chat_id = %self.chat_id,
                        retry = retry_count,
                        "Calling OpenAI agent.stream_chat"
                    );
                    let stream = openai_agent.stream_chat(&prompt, history.clone()).await;
                    tracing::info!(
                        chat_id = %self.chat_id,
                        "Stream created (OpenAI), entering response loop"
                    );
                    self.process_agent_stream(stream, &cancellation_token, &mut conn, &session, &mut item_count).await
                }
                Agent::OpenRouter(openrouter_agent) => {
                    tracing::info!(
                        chat_id = %self.chat_id,
                        retry = retry_count,
                        "Calling OpenRouter agent.stream_chat"
                    );
                    let stream = openrouter_agent.stream_chat(&prompt, history.clone()).await;
                    tracing::info!(
                        chat_id = %self.chat_id,
                        "Stream created (OpenRouter), entering response loop"
                    );
                    self.process_agent_stream(stream, &cancellation_token, &mut conn, &session, &mut item_count).await
                }
            };

            match result {
                Ok(response) => break response,
                Err(e) => {
                    // Check if error is retryable (transient errors)
                    let error_str = format!("{:?}", e);
                    let is_retryable = error_str.contains("Failed to get tool definitions")
                        || error_str.contains("RequestError")
                        || error_str.contains("rate limit")
                        || error_str.contains("timeout")
                        || error_str.contains("connection")
                        || error_str.contains("5")
                        || error_str.contains("overloaded");

                    if is_retryable && retry_count < MAX_AI_RETRIES {
                        retry_count += 1;
                        let backoff_ms = RETRY_BACKOFF_MS * (1 << (retry_count - 1)); // Exponential backoff
                        tracing::warn!(
                            chat_id = %self.chat_id,
                            retry = retry_count,
                            max_retries = MAX_AI_RETRIES,
                            backoff_ms = backoff_ms,
                            error = ?e,
                            "[ChatActor] Transient AI error, retrying with backoff"
                        );

                        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    } else {
                        // Non-retryable error or max retries exceeded
                        tracing::error!(
                            chat_id = %self.chat_id,
                            retry = retry_count,
                            error = ?e,
                            "[ChatActor] AI error (not retrying)"
                        );
                        return Err(e);
                    }
                }
            }
        };

        // Check if stream completed without any items (possible API access issue)
        if item_count == 0 {
            tracing::warn!(
                "[ChatActor] [Rig] Stream completed with 0 items for chat {} (model: {}). This may indicate an API access issue or invalid model name.",
                self.chat_id,
                session.agent_config.model
            );
        }

        // Remove cancellation token - stream is complete
        self.registry.remove_cancellation(&self.chat_id).await;

        // Tool action log display removed - audit trail captures all interactions

        // 7. Save Assistant Response
        if !full_response.is_empty() {
            tracing::info!(
                "[ChatActor] Saving AI response to database for chat {} (model: {}, length={})",
                self.chat_id,
                session.agent_config.model,
                full_response.len()
            );
            let mut final_conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

            // Flush any remaining reasoning buffer before saving final response
            if let Err(e) = self.flush_reasoning_buffer(&mut final_conn).await {
                tracing::error!(
                    chat_id = %self.chat_id,
                    error = %e,
                    "[ChatActor] Failed to flush reasoning buffer before final response"
                );
            }

            let reasoning_id = self.state.lock().await.current_reasoning_id.clone();

            ChatService::save_message(
                &mut final_conn,
                &self.storage,
                self.workspace_id,
                NewChatMessage {
                    file_id: self.chat_id,
                    workspace_id: self.workspace_id,
                    role: ChatMessageRole::Assistant,
                    content: full_response,
                    metadata: sqlx::types::Json(crate::models::chat::ChatMessageMetadata {
                        model: Some(session.agent_config.model.clone()),
                        reasoning_id,
                        ..Default::default()
                    }),
                },
            )
            .await?;
        }

        let send_result = self.event_tx.send(SseEvent::Done {
            message: "Turn complete".to_string(),
        });
        if let Err(e) = send_result {
            tracing::error!(
                chat_id = %self.chat_id,
                event_type = "Done",
                error = ?e,
                receivers = self.event_tx.receiver_count(),
                "[SSE] FAILED to send event - no receivers"
            );
        } else {
            tracing::debug!(
                chat_id = %self.chat_id,
                event_type = "Done",
                receivers = self.event_tx.receiver_count(),
                "[SSE] SENT event successfully"
            );
        }

        tracing::info!(
            chat_id = %self.chat_id,
            receivers = self.event_tx.receiver_count(),
            "[ChatActor] ProcessInteraction COMPLETED"
        );

        // Clear the cancellation token for this interaction
        self.state.lock().await.interaction.current_cancellation_token = None;

        // Clear reasoning ID for next interaction
        self.state.lock().await.current_reasoning_id = None;

        // Clear current task (interaction complete)
        let current_task = self.state.lock().await.interaction.current_task.take();
        if let Some(task) = current_task {
            tracing::debug!(
                chat_id = %self.chat_id,
                task = %task,
                "[ChatActor] Cleared current task (interaction complete)"
            );
        }

        Ok(())
    }

    async fn handle_cancellation(
        &self,
        conn: &mut sqlx::PgConnection,
        partial_response: String,
        reason: &str,
    ) -> crate::error::Result<()> {
        tracing::info!(
            "[ChatActor] Handling cancellation for chat {} (reason: {})",
            self.chat_id,
            reason
        );

        // Flush any pending reasoning buffer before cancellation persistence
        if let Err(e) = self.flush_reasoning_buffer(conn).await {
            tracing::error!(
                chat_id = %self.chat_id,
                error = %e,
                "[ChatActor] Failed to flush reasoning buffer during cancellation"
            );
        }

        // Remove cancellation token - stream is being cancelled
        self.registry.remove_cancellation(&self.chat_id).await;

        // Get current model for metadata
        let model = self.state.lock().await.interaction.current_model.clone()
            .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());

        // 1. Send Stopped event to all SSE clients
        let _ = self.event_tx.send(SseEvent::Stopped {
            reason: reason.to_string(),
            partial_response: if partial_response.is_empty() {
                None
            } else {
                Some(partial_response.clone())
            },
        });

        // 2. Save partial response if there is any text
        if !partial_response.is_empty() {
            tracing::info!(
                "[ChatActor] Saving partial response ({} chars) for chat {} (model: {})",
                partial_response.len(),
                self.chat_id,
                model
            );
            self.save_partial_response(conn, partial_response.clone(), model).await?;
        }

        // 3. Add cancellation marker to chat history for AI awareness
        self.add_cancellation_marker(conn, reason).await?;

        // 4. Clear the cancellation token for this interaction
        self.state.lock().await.interaction.current_cancellation_token = None;

        // 5. Clear reasoning ID for next interaction
        self.state.lock().await.current_reasoning_id = None;

        // 7. Clear processing flag - actor is no longer actively processing
        self.state.lock().await.interaction.is_actively_processing = false;

        Ok(())
    }

    async fn save_partial_response(
        &self,
        conn: &mut sqlx::PgConnection,
        content: String,
        model: String,
    ) -> crate::error::Result<()> {
        ChatService::save_message(
            conn,
            &self.storage,
            self.workspace_id,
            NewChatMessage {
                file_id: self.chat_id,
                workspace_id: self.workspace_id,
                role: ChatMessageRole::Assistant,
                content,
                metadata: sqlx::types::Json(crate::models::chat::ChatMessageMetadata {
                    model: Some(model),
                    ..Default::default()
                }),
            },
        )
        .await?;
        Ok(())
    }

    async fn add_cancellation_marker(
        &self,
        conn: &mut sqlx::PgConnection,
        reason: &str,
    ) -> crate::error::Result<()> {
        let marker_content = format!(
            "[System: Response was interrupted by user ({})]",
            reason
        );

        ChatService::save_message(
            conn,
            &self.storage,
            self.workspace_id,
            NewChatMessage {
                file_id: self.chat_id,
                workspace_id: self.workspace_id,
                role: ChatMessageRole::System,
                content: marker_content,
                metadata: sqlx::types::Json(crate::models::chat::ChatMessageMetadata::default()),
            },
        )
        .await?;
        Ok(())
    }

    /// Process a single stream item from either provider
    /// This method is generic over the stream response type to work with both OpenAI and OpenRouter
    async fn process_stream_item<M>(
        &self,
        stream_item: rig::agent::MultiTurnStreamItem<M>,
        full_response: &mut String,
        has_started_responding: &mut bool,
        item_count: usize,
        conn: &mut sqlx::PgConnection,
        _session: &crate::models::chat::ChatSession,
        _cancellation_token: &CancellationToken,
    ) -> crate::error::Result<()>
    where
        M: std::fmt::Debug + 'static,
    {
        match stream_item {
            rig::agent::MultiTurnStreamItem::StreamAssistantItem(content) => {
                match content {
                    rig::streaming::StreamedAssistantContent::Text(text) => {
                        // Flush pending reasoning buffer before text chunk
                        if let Err(e) = self.flush_reasoning_buffer(conn).await {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                error = %e,
                                "[ChatActor] Failed to flush reasoning buffer before text"
                            );
                        }

                        tracing::debug!(
                            chat_id = %self.chat_id,
                            text_len = text.text.len(),
                            text_preview = %crate::utils::safe_preview(&text.text, 50),
                            "[ChatActor] [Rig] Received Text chunk"
                        );
                        if !*has_started_responding {
                            tracing::info!("[ChatActor] [Rig] AI started streaming text response for chat {}", self.chat_id);
                            *has_started_responding = true;
                        }
                        full_response.push_str(&text.text);
                        let send_result = self.event_tx.send(SseEvent::Chunk { text: text.text.clone() });
                        if let Err(e) = send_result {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                error = %e,
                                receivers = self.event_tx.receiver_count(),
                                "[SSE] Failed to send Chunk event - no receivers or broadcast channel closed"
                            );
                        } else {
                            tracing::debug!(
                                chat_id = %self.chat_id,
                                text_len = text.text.len(),
                                receivers = self.event_tx.receiver_count(),
                                "[SSE] Successfully sent Chunk event"
                            );
                        }
                    }
                    rig::streaming::StreamedAssistantContent::ReasoningDelta { id, reasoning } => {
                        tracing::debug!(
                            chat_id = %self.chat_id,
                            reasoning_len = reasoning.len(),
                            id = ?id,
                            "[ChatActor] [Rig] Received streaming ReasoningDelta chunk"
                        );

                        // Generate reasoning_id on first chunk to link all chunks from this turn
                        {
                            let mut state = self.state.lock().await;
                            state.ensure_reasoning_id();
                            // Buffer reasoning chunk (will be saved aggregated later)
                            state.reasoning_buffer.push(reasoning.clone());
                            tracing::debug!(
                                chat_id = %self.chat_id,
                                buffer_size = state.reasoning_buffer.len(),
                                "[ChatActor] Buffered ReasoningDelta chunk"
                            );
                        }

                        // Send streaming reasoning chunk to frontend via Thought event
                        let send_result = self.event_tx.send(SseEvent::Thought {
                            agent_id: None,
                            text: reasoning.clone(),
                        });
                        if let Err(e) = send_result {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                event_type = "Thought",
                                error = ?e,
                                receivers = self.event_tx.receiver_count(),
                                "[SSE] FAILED to send event - no receivers"
                            );
                        } else {
                            tracing::trace!(
                                chat_id = %self.chat_id,
                                event_type = "Thought",
                                reasoning_len = reasoning.len(),
                                receivers = self.event_tx.receiver_count(),
                                "[SSE] SENT event successfully"
                            );
                        }
                    }
                    rig::streaming::StreamedAssistantContent::Reasoning(thought) => {
                        tracing::info!(
                            chat_id = %self.chat_id,
                            reasoning_parts = thought.reasoning.len(),
                            "[ChatActor] [Rig] Received final Reasoning (accumulated)"
                        );

                        // Buffer all reasoning parts
                        {
                            let mut state = self.state.lock().await;
                            state.ensure_reasoning_id();
                            for part in &thought.reasoning {
                                if !part.trim().is_empty() {
                                    state.reasoning_buffer.push(part.clone());
                                }
                            }
                        }

                        // Send to frontend
                        for part in &thought.reasoning {
                            if !part.trim().is_empty() {
                                let send_result = self.event_tx.send(SseEvent::Thought {
                                    agent_id: None,
                                    text: part.clone(),
                                });
                                if let Err(e) = send_result {
                                    tracing::error!(
                                        chat_id = %self.chat_id,
                                        event_type = "Thought",
                                        error = ?e,
                                        receivers = self.event_tx.receiver_count(),
                                        "[SSE] FAILED to send event - no receivers"
                                    );
                                } else {
                                    tracing::trace!(
                                        chat_id = %self.chat_id,
                                        event_type = "Thought",
                                        reasoning_len = part.len(),
                                        receivers = self.event_tx.receiver_count(),
                                        "[SSE] SENT event successfully"
                                    );
                                }
                            }
                        }

                        // Flush aggregated reasoning to database
                        if let Err(e) = self.flush_reasoning_buffer(conn).await {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                error = %e,
                                "[ChatActor] Failed to flush reasoning buffer"
                            );
                        }
                    }
                    rig::streaming::StreamedAssistantContent::ToolCall(tool_call) => {
                        // Flush pending reasoning buffer before tool call
                        if let Err(e) = self.flush_reasoning_buffer(conn).await {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                error = %e,
                                "[ChatActor] Failed to flush reasoning buffer before tool call"
                            );
                        }

                        tracing::info!("[ChatActor] [Rig] AI calling tool {} for chat {}", tool_call.function.name, self.chat_id);

                        // Extract arguments as JSON for persistence
                        let arguments_json = match serde_json::to_value(&tool_call.function.arguments) {
                            Ok(val) => val,
                            Err(e) => {
                                tracing::warn!(
                                    chat_id = %self.chat_id,
                                    tool = %tool_call.function.name,
                                    error = %e,
                                    "Failed to serialize tool arguments for persistence"
                                );
                                serde_json::json!({ "error": "Failed to serialize arguments" })
                            }
                        };

                        // Summarize arguments for persistence to avoid DB bloat
                        let summarized_args = ChatService::summarize_tool_inputs(
                            &tool_call.function.name,
                            &arguments_json,
                        );

                        // Build detailed content for .chat file
                        let args_preview = if let Ok(args_str) = serde_json::to_string_pretty(&summarized_args) {
                            // Truncate if too long for file content
                            if args_str.len() > 500 {
                                format!("{}...\n[Arguments truncated, see metadata for full details]", crate::utils::truncate_safe(&args_str, 500))
                            } else {
                                args_str
                            }
                        } else {
                            "[Could not serialize arguments]".to_string()
                        };
                        let tool_call_content = format!(
                            "AI called tool: {}\nArguments:\n{}",
                            tool_call.function.name,
                            args_preview
                        );

                        // Persist tool call for audit trail
                        let reasoning_id = {
                            let mut state = self.state.lock().await;
                            state.ensure_reasoning_id();
                            state.current_reasoning_id.clone()
                        };
                        let metadata = ChatMessageMetadata {
                            message_type: Some("tool_call".to_string()),
                            reasoning_id,
                            tool_name: Some(tool_call.function.name.clone()),
                            tool_arguments: Some(summarized_args),
                            ..Default::default()
                        };
                        if let Err(e) = ChatService::save_stream_event(
                            conn,
                            &self.storage,
                            self.workspace_id,
                            self.chat_id,
                            ChatMessageRole::Tool,
                            tool_call_content,
                            metadata,
                        ).await {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                tool = %tool_call.function.name,
                                error = %e,
                                "[ChatActor] Failed to persist tool call"
                            );
                        }

                        // Track tool name and arguments for logging when ToolResult arrives
                        {
                            let mut state = self.state.lock().await;
                            state.tool_tracking.current_tool_name = Some(tool_call.function.name.clone());
                            state.tool_tracking.current_tool_args = Some(arguments_json);
                        }

                        let path = tool_call.function.arguments.get("path")
                            .or_else(|| tool_call.function.arguments.get("source"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let tool_name = tool_call.function.name.clone();
                        if let Err(e) = self.event_tx.send(SseEvent::Call {
                            tool: tool_call.function.name,
                            path,
                            args: tool_call.function.arguments,
                        }) {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                event_type = "Call",
                                tool = %tool_name,
                                error = ?e,
                                receivers = self.event_tx.receiver_count(),
                                "[SSE] FAILED to send event - no receivers"
                            );
                        } else {
                            tracing::debug!(
                                chat_id = %self.chat_id,
                                event_type = "Call",
                                tool = %tool_name,
                                receivers = self.event_tx.receiver_count(),
                                "[SSE] SENT event successfully"
                            );
                        }
                    }
                    _ => {}
                }
            }
            rig::agent::MultiTurnStreamItem::StreamUserItem(content) => {
                match content {
                    rig::streaming::StreamedUserContent::ToolResult(result) => {
                        // Flush reasoning buffer before tool result persistence
                        if let Err(e) = self.flush_reasoning_buffer(conn).await {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                error = %e,
                                "[ChatActor] Failed to flush reasoning buffer before tool result"
                            );
                        }

                        let output = if let Some(rig::completion::message::ToolResultContent::Text(text)) = result.content.iter().next() {
                            text.text.clone()
                        } else {
                            "Tool execution completed".to_string()
                        };

                        // Determine tool success by parsing the response
                        // Tools return ToolResponse {success, result, error} format
                        // Define error detection heuristic once to avoid duplication
                        // Only check for "ToolCallError" to avoid false positives from tool result content
                        let has_error_heuristic = |s: &str| {
                            s.contains("ToolCallError")
                        };

                        let (success, normalized_output) = if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&output) {
                            // Check if this is a ToolResponse format with explicit success field
                            if let Some(success_bool) = result_json.get("success").and_then(|v| v.as_bool()) {
                                // ToolResponse format: {"success": true/false, "result": ..., "error": ...}
                                let normalized = if success_bool {
                                    // Success: extract result field
                                    result_json.get("result")
                                        .cloned()
                                        .unwrap_or(result_json)
                                        .to_string()
                                } else {
                                    // Failure: extract error message
                                    result_json.get("error")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| output.clone())
                                };
                                (success_bool, normalized)
                            } else {
                                // No success field - likely a bare result (success)
                                // Check for error patterns in the output
                                let has_error = has_error_heuristic(&output);
                                (!has_error, output.clone())
                            }
                        } else {
                            // Not JSON - use heuristic for plain text
                            let has_error = has_error_heuristic(&output);
                            (!has_error, output.clone())
                        };

                        // Check if this is an ask_user tool result with question_pending
                         // Only ask_user returns question_pending status
                         if success {
                             if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&normalized_output) {
                                 // Handle question_pending (ask_user tool)
                                 if let Some(status) = result_json.get("status").and_then(|s| s.as_str()) {
                                     if status == "question_pending" {
                                         // Extract question data
                                         if let Some(questions) = result_json.get("questions").and_then(|q| q.as_array()) {
                                             let parsed_questions: Vec<crate::models::sse::Question> = questions
                                                 .iter()
                                                 .filter_map(|q| serde_json::from_value(q.clone()).ok())
                                                 .collect();

                                             if let Some(question_id) = result_json.get("question_id").and_then(|id| id.as_str()) {
                                                 if let Ok(qid) = uuid::Uuid::parse_str(question_id) {
                                                     tracing::info!(
                                                         "[ChatActor] Emitting QuestionPending event with {} questions for chat {}",
                                                         parsed_questions.len(),
                                                         self.chat_id
                                                     );
                                                     let _ = self.event_tx.send(SseEvent::QuestionPending {
                                                         question_id: qid,
                                                         questions: parsed_questions,
                                                         created_at: chrono::Utc::now(),
                                                     });
                                                 }
                                             }
                                         }
                                     }
                                 }

                                 // Handle mode transition (exit_plan_mode tool)
                                 // Check if result has mode field = "build"
                                 if let Some(mode) = result_json.get("mode").and_then(|m| m.as_str()) {
                                     if mode == "build" {
                                         let plan_file = result_json.get("plan_file")
                                             .and_then(|p| p.as_str())
                                             .unwrap_or("");

                                         tracing::info!(
                                             "[ChatActor] exit_plan_mode succeeded, transitioning chat {} to Build Mode with plan file {}",
                                             self.chat_id,
                                             plan_file
                                         );

                                         // Update chat metadata in database using ChatService
                                         // This properly creates a new FileVersion and commits the transaction
                                         // CRITICAL: If this fails, we must propagate the error to prevent state mismatch
                                         let mut update_conn = self.pool.acquire().await
                                             .map_err(|e| {
                                                 tracing::error!("[ChatActor] Failed to acquire DB connection for metadata update: {:?}", e);
                                                 crate::error::Error::Sqlx(e)
                                             })?;
                                         ChatService::update_chat_metadata(
                                             &mut update_conn,
                                             &self.storage,
                                             self.workspace_id,
                                             self.chat_id,
                                             mode.to_string(),
                                             if plan_file.is_empty() { None } else { Some(plan_file.to_string()) },
                                         ).await.map_err(|e| {
                                             tracing::error!("[ChatActor] Failed to update chat metadata: {:?}", e);
                                             e
                                         })?;

                                         tracing::info!(
                                             "[ChatActor] Successfully updated chat {} metadata: mode={}, plan_file={}",
                                             self.chat_id,
                                             mode,
                                             plan_file
                                         );

                                         // Update agent session metadata with new mode and agent type
                                         if let Some(session_id) = self.session_id {
                                             let agent_type = crate::models::agent_session::AgentType::Builder;
                                             if let Err(e) = agent_sessions::update_session_metadata(
                                                 &mut update_conn,
                                                 session_id,
                                                 None,  // model unchanged
                                                 Some(mode.to_string()),
                                                 Some(agent_type),
                                                 self.user_id,
                                             ).await {
                                                 tracing::warn!(
                                                     session_id = %session_id,
                                                     error = %e,
                                                     "[ChatActor] Failed to update session metadata after mode change"
                                                 );
                                             } else {
                                                 tracing::info!(
                                                     session_id = %session_id,
                                                     mode = %mode,
                                                     agent_type = %agent_type,
                                                     "[ChatActor] Successfully updated session metadata after mode change"
                                                 );
                                             }
                                         }

                                         // Emit mode_changed event to frontend
                                         let _ = self.event_tx.send(SseEvent::ModeChanged {
                                             mode: "build".to_string(),
                                             plan_file: if plan_file.is_empty() { None } else { Some(plan_file.to_string()) },
                                         });
                                     }
                                 }
                             }
                         }

                         tracing::info!(
                             "[ChatActor] [Rig] Tool execution finished for chat {} (success: {}). Output: {}",
                             self.chat_id,
                             success,
                             crate::utils::safe_preview(&output, 100)
                         );
                          let send_result = self.event_tx.send(SseEvent::Observation { output: normalized_output.clone(), success });
                          if let Err(e) = send_result {
                              tracing::error!(
                                  chat_id = %self.chat_id,
                                  event_type = "Observation",
                                  error = ?e,
                                  receivers = self.event_tx.receiver_count(),
                                  "[SSE] FAILED to send event - no receivers"
                              );
                          } else {
                              tracing::debug!(
                                  chat_id = %self.chat_id,
                                  event_type = "Observation",
                                  success = success,
                                  receivers = self.event_tx.receiver_count(),
                                  "[SSE] SENT event successfully"
                              );
                          }


                          // Persist tool result for audit trail (BEFORE any state modifications)
                          {
                              let (tool_name_opt, reasoning_id) = {
                                  let state = self.state.lock().await;
                                  (state.tool_tracking.current_tool_name.clone(), state.current_reasoning_id.clone())
                              };
                              if let Some(tool_name) = tool_name_opt {
                                  // Summarize output for persistence to avoid DB bloat
                                  let summarized_output = ChatService::summarize_tool_outputs(&tool_name, &normalized_output);

                                  // Build detailed content for .chat file
                                  let tool_result_content = format!(
                                      "Tool {}: {}\nOutput:\n{}",
                                      tool_name,
                                      if success { "succeeded" } else { "failed" },
                                      summarized_output
                                  );

                                  let metadata = ChatMessageMetadata {
                                      message_type: Some("tool_result".to_string()),
                                      reasoning_id,
                                      tool_name: Some(tool_name.clone()),
                                      tool_output: Some(summarized_output),
                                      tool_success: Some(success),
                                      ..Default::default()
                                  };
                                  if let Err(e) = ChatService::save_stream_event(
                                      conn,
                                      &self.storage,
                                      self.workspace_id,
                                      self.chat_id,
                                      ChatMessageRole::Tool,
                                      tool_result_content,
                                      metadata,
                                  ).await {
                                      tracing::error!(
                                          chat_id = %self.chat_id,
                                          tool = %tool_name,
                                          error = %e,
                                          "[ChatActor] Failed to persist tool result"
                                      );
                                  }
                              } else {
                                  tracing::warn!(
                                      chat_id = %self.chat_id,
                                      "[ChatActor] Tool result without tracked tool name; skipping persistence"
                                  );
                              }
                          }

                          // Tool action logs removed - audit trail captures all interactions

                         // Clear current tool name and arguments
                         {
                             let mut state = self.state.lock().await;
                             state.tool_tracking.current_tool_name = None;
                             state.tool_tracking.current_tool_args = None;
                         }
                    }
                }
            }
            rig::agent::MultiTurnStreamItem::FinalResponse(final_response) => {
                // Flush reasoning buffer before final response
                if let Err(e) = self.flush_reasoning_buffer(conn).await {
                    tracing::error!(
                        chat_id = %self.chat_id,
                        error = %e,
                        "[ChatActor] Failed to flush reasoning buffer before final response"
                    );
                }

                tracing::info!(
                    chat_id = %self.chat_id,
                    response_len = final_response.response().len(),
                    response_text = %final_response.response(),
                    usage = ?final_response.usage(),
                    "Received FinalResponse from stream"
                );

                // Note: FinalResponse contains the complete text, but we DON'T append it
                // because full_response has already accumulated all Text chunks during streaming.
                // Appending would cause duplication in the saved message.
                //
                // FinalResponse is only used here for logging and usage statistics.
                let response_text = final_response.response();

                // Debug logging to diagnose duplication issues
                tracing::debug!(
                    chat_id = %self.chat_id,
                    full_response_len = full_response.len(),
                    final_response_len = response_text.len(),
                    "[ChatActor] FinalResponse received - accumulated={} vs final={}",
                    full_response.len(),
                    response_text.len()
                );

                if !response_text.is_empty() && !*has_started_responding {
                    tracing::info!("[ChatActor] AI started responding (via FinalResponse) for chat {}", self.chat_id);
                    *has_started_responding = true;
                }
            }
            // Catch-all for future Rig variants (MultiTurnStreamItem is non-exhaustive)
            _ => {
                tracing::warn!(
                    chat_id = %self.chat_id,
                    item_num = item_count,
                    "Unhandled stream item variant"
                );
            }
        }

        Ok(())
    }

    /// Process a generic agent stream, handling cancellation and stream items
    async fn process_agent_stream<S, M, E>(
        &self,
        mut stream: S,
        cancellation_token: &CancellationToken,
        conn: &mut sqlx::PgConnection,
        session: &crate::models::chat::ChatSession,
        item_count: &mut usize,
    ) -> Result<String>
    where
        S: futures::Stream<Item = std::result::Result<rig::agent::MultiTurnStreamItem<M>, E>> + Unpin,
        M: std::fmt::Debug + 'static,
        E: std::fmt::Display,
    {
        let mut full_response = String::new();
        let mut has_started_responding = false;

        loop {
            // Check for cancellation before each stream iteration
            if cancellation_token.is_cancelled() {
                tracing::info!("[ChatActor] Cancelled during streaming for chat {}", self.chat_id);
                self.handle_cancellation(conn, full_response.clone(), "user_cancelled").await?;
                return Err(crate::error::Error::Internal("Chat cancelled by user".to_string()));
            }

            // Use tokio::select! to allow cancellation during stream.next()
            // Also add a timeout to detect stalled API streams
            let stream_timeout = tokio::time::Duration::from_secs(STREAM_READ_TIMEOUT_SECS);
            let item = tokio::select! {
                _ = cancellation_token.cancelled() => {
                    tracing::info!("[ChatActor] Cancelled during streaming for chat {}", self.chat_id);
                    self.handle_cancellation(conn, full_response.clone(), "user_cancelled").await?;
                    return Err(crate::error::Error::Internal("Chat cancelled by user".to_string()));
                },
                item_result = tokio::time::timeout(stream_timeout, stream.next()) => {
                    match item_result {
                        Ok(item) => {
                            match &item {
                                Some(Ok(_)) => tracing::debug!(chat_id = %self.chat_id, item_num = *item_count, "Received stream item"),
                                Some(Err(e)) => tracing::error!(chat_id = %self.chat_id, error = %e, "Stream error"),
                                None => tracing::info!(chat_id = %self.chat_id, "Stream ended (None)"),
                            }
                            item
                        }
                        Err(_) => {
                            // Timeout - API stream stalled
                            tracing::warn!(
                                chat_id = %self.chat_id,
                                timeout_secs = STREAM_READ_TIMEOUT_SECS,
                                "[ChatActor] Stream read timeout - API stalled"
                            );
                            return Err(crate::error::Error::Internal(
                                format!("Stream read timeout after {} seconds - API stalled", STREAM_READ_TIMEOUT_SECS)
                            ));
                        }
                    }
                }
            };

            let item = match item {
                Some(i) => i,
                None => {
                    tracing::info!(chat_id = %self.chat_id, items_received = *item_count, "Stream finished naturally");
                    break;
                }
            };

            *item_count += 1;

            match item {
                Err(e) => {
                    let error_str = e.to_string();

                    // Check for JSON parsing errors in tool calls (common with very long content)
                    if error_str.contains("JsonError") || error_str.contains("EOF while parsing") {
                        tracing::error!(
                            chat_id = %self.chat_id,
                            error = %error_str,
                            item_num = *item_count,
                            "[ChatActor] JSON parsing error in tool call - content may be too long or have invalid characters"
                        );
                        return Err(crate::error::Error::Internal(
                            "Tool call JSON parsing failed. The content may be too long or contain invalid characters. \
                             Try using smaller content chunks or check for special characters that need escaping.".to_string()
                        ));
                    }

                    tracing::error!(
                        chat_id = %self.chat_id,
                        error = %e,
                        item_num = *item_count,
                        "Stream item error"
                    );
                    return Err(crate::error::Error::Internal(format!("Streaming error: {}", e)));
                }
                Ok(stream_item) => {
                    if let Err(e) = self.process_stream_item(
                        stream_item,
                        &mut full_response,
                        &mut has_started_responding,
                        *item_count,
                        conn,
                        session,
                        cancellation_token,
                    ).await {
                        return Err(e);
                    }
                }
            }
        }

        Ok(full_response)
    }

    async fn flush_reasoning_buffer(
        &self,
        conn: &mut sqlx::PgConnection,
    ) -> crate::error::Result<()> {
        let (buffer, reasoning_id) = {
            let mut state = self.state.lock().await;
            if state.reasoning_buffer.is_empty() {
                return Ok(());
            }
            // Atomically take the buffer, leaving an empty one in its place to prevent a race condition.
            let buffer = std::mem::take(&mut state.reasoning_buffer);
            let reasoning_id = state.ensure_reasoning_id();
            (buffer, reasoning_id)
        };

        let aggregated_reasoning = buffer.join("");
        if aggregated_reasoning.is_empty() {
            return Ok(());
        }

        tracing::debug!(
            chat_id = %self.chat_id,
            reasoning_len = aggregated_reasoning.len(),
            reasoning_id = %reasoning_id,
            "[ChatActor] Flushing aggregated reasoning buffer to DB"
        );

        let metadata = ChatMessageMetadata {
            message_type: Some("reasoning_complete".to_string()),
            reasoning_id: Some(reasoning_id),
            ..Default::default()
        };

        ChatService::save_stream_event(
            conn,
            &self.storage,
            self.workspace_id,
            self.chat_id,
            ChatMessageRole::Assistant,
            aggregated_reasoning,
            metadata,
        )
        .await?;

        Ok(())
    }

    async fn get_or_create_agent(
        &self,
        user_id: Uuid,
        session: &crate::models::chat::ChatSession,
        ai_config: &crate::config::AiConfig,
    ) -> crate::error::Result<Agent> {
        tracing::info!(
            "[ChatActor] Creating new agent for chat {} (model: {})",
            self.chat_id, session.agent_config.model
        );

        // Create fresh agent - no caching, simpler and always correct
        let agent = self.rig_service.create_agent(
            self.pool.clone(),
            self.storage.clone(),
            self.workspace_id,
            self.chat_id,
            user_id,
            session,
            ai_config,
        ).await?;

        // Update session metadata with the actual model being used
        if let Some(session_id) = &self.session_id {
            tracing::debug!(
                session_id = %session_id,
                model = %session.agent_config.model,
                mode = %session.agent_config.mode,
                "[ChatActor] Updating session metadata after agent creation"
            );

            let mut conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

            // Determine agent type from mode
            let agent_type = match session.agent_config.mode.as_str() {
                "plan" => Some(AgentType::Planner),
                "build" => Some(AgentType::Builder),
                _ => Some(AgentType::Assistant),
            };

            // Update session with actual model, mode, and agent type
            if let Err(e) = agent_sessions::update_session_metadata(
                &mut conn,
                *session_id,
                Some(session.agent_config.model.clone()),
                Some(session.agent_config.mode.clone()),
                agent_type,
                self.user_id,
            ).await {
                tracing::warn!(
                    session_id = %session_id,
                    error = %e,
                    "[ChatActor] Failed to update session metadata after agent creation"
                );
            } else {
                tracing::info!(
                    session_id = %session_id,
                    model = %session.agent_config.model,
                    mode = %session.agent_config.mode,
                    "[ChatActor] Successfully updated session metadata"
                );
            }
        }

        Ok(agent)
    }

    // ========================================================================
    // SESSION TRACKING METHODS
    // ========================================================================

    /// Creates a new agent session in the database.
    async fn create_session(&self) -> Result<Uuid> {
        tracing::info!(
            chat_id = %self.chat_id,
            workspace_id = %self.workspace_id,
            user_id = %self.user_id,
            persona = %self.default_persona,
            "[ChatActor] Creating agent session in database"
        );

        let mut conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

        // Get the chat file's latest version to extract actual model and mode
        // This ensures the session is created with the correct values from the chat config
        let (actual_model, actual_mode) = match queries::files::get_latest_version(&mut conn, self.chat_id).await {
            Ok(version) => {
                // Extract model and mode from app_data
                let model = version.app_data.get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or(DEFAULT_CHAT_MODEL)
                    .to_string();

                let mode = version.app_data.get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("plan")
                    .to_string();

                tracing::debug!(
                    chat_id = %self.chat_id,
                    model = %model,
                    mode = %mode,
                    "[ChatActor] Extracted model and mode from chat file app_data"
                );

                (model, mode)
            }
            Err(e) => {
                tracing::warn!(
                    chat_id = %self.chat_id,
                    error = %e,
                    "[ChatActor] Failed to get chat file version, using defaults"
                );
                (DEFAULT_CHAT_MODEL.to_string(), "plan".to_string())
            }
        };

        // Determine agent type from mode (not from persona)
        let agent_type = match actual_mode.as_str() {
            "plan" => AgentType::Planner,
            "build" => AgentType::Builder,
            _ => AgentType::Assistant,
        };

        tracing::debug!(
            chat_id = %self.chat_id,
            agent_type = %agent_type,
            model = %actual_model,
            mode = %actual_mode,
            "[ChatActor] Session configuration determined from chat file"
        );

        let session = agent_sessions::get_or_create_session(
            &mut conn,
            self.workspace_id,
            self.chat_id,
            self.user_id,
            agent_type,
            actual_model,
            actual_mode,
        )
        .await?;

        tracing::info!(
            chat_id = %self.chat_id,
            session_id = %session.id,
            model = %session.model,
            mode = %session.mode,
            agent_type = %session.agent_type,
            "[ChatActor] Successfully created agent session with correct values"
        );

        Ok(session.id)
    }

    /// Updates the session status in the database.
    async fn update_session_status(
        &self,
        session_id: Uuid,
        status: crate::models::agent_session::SessionStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        tracing::info!(
            chat_id = %self.chat_id,
            session_id = %session_id,
            new_status = %status,
            error_message = ?error_message,
            "[ChatActor] update_session_status: Starting update"
        );

        let mut conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

        let _ = queries::agent_sessions::update_session_status(&mut conn, session_id, status, error_message).await?;

        tracing::info!(
            chat_id = %self.chat_id,
            session_id = %session_id,
            new_status = %status,
            "[ChatActor] update_session_status: Successfully updated"
        );

        Ok(())
    }

    /// Starts a background task that sends periodic heartbeats to the database.
    /// This keeps the session alive and indicates the agent is actively running.
    fn start_heartbeat_task(&self, session_id: Uuid) -> JoinHandle<()> {
        tracing::info!(
            chat_id = %self.chat_id,
            session_id = %session_id,
            "[ChatActor] Starting heartbeat task (30s interval)"
        );

        let pool = self.pool.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                tracing::trace!(
                    session_id = %session_id,
                    "[ChatActor] Heartbeat: sending update"
                );

                let mut conn = match pool.acquire().await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!(
                            session_id = %session_id,
                            error = %e,
                            "[ChatActor] Heartbeat: failed to acquire database connection"
                        );
                        continue;
                    }
                };

                // Update heartbeat timestamp
                if let Err(e) = agent_sessions::update_heartbeat(&mut conn, session_id).await {
                    tracing::warn!(
                        session_id = %session_id,
                        error = %e,
                        "[ChatActor] Heartbeat: failed to update heartbeat"
                    );
                } else {
                    tracing::trace!(
                        session_id = %session_id,
                        "[ChatActor] Heartbeat: successfully updated"
                    );
                }
            }
        })
    }

    // ========================================================================
    // STATE MACHINE INTEGRATION METHODS
    // ========================================================================

    /// Log a state transition for debugging and monitoring
    #[instrument(skip(self), fields(chat_id = %self.chat_id))]
    fn log_state_transition(&self, from: ActorState, to: ActorState, reason: &str) {
        info!(
            from_state = %from,
            to_state = %to,
            reason = %reason,
            "[ChatActor] State transition"
        );

        // Emit StateChanged SSE event
        let _ = self.event_tx.send(SseEvent::StateChanged {
            from_state: from.to_string(),
            to_state: to.to_string(),
            reason: Some(reason.to_string()),
        });
    }

    /// Get the current state from the state machine
    fn current_state(&self) -> ActorState {
        self.state_machine.current_state()
    }

    /// Transition state using the state machine and log the transition
    #[instrument(skip(self, event, reason), fields(chat_id = %self.chat_id))]
    async fn transition_state(
        &mut self,
        event: ActorEvent,
        reason: &str,
    ) -> Result<()> {
        let from_state = self.current_state();

        // Try to perform the transition
        match self.state_machine.handle_event(event) {
            Ok(transition) if transition.state_changed => {
                self.log_state_transition(from_state, transition.new_state, reason);
                Ok(())
            }
            Ok(_) => {
                // No state change, but not an error
                Ok(())
            }
            Err(e) => {
                warn!(
                    from_state = %from_state,
                    error = ?e,
                    "[ChatActor] State transition failed, continuing with current state"
                );
                // Continue despite failed transition - don't break the actor
                Ok(())
            }
        }
    }

    /// Execute a state action returned by a state handler.
    #[instrument(skip(self), fields(chat_id = %self.chat_id))]
    async fn execute_state_action(&mut self, action: StateAction) -> Result<()> {
        match action {
            StateAction::UpdateSessionStatus(status) => {
                if let Some(session_id) = self.session_id {
                    tracing::debug!(
                        chat_id = %self.chat_id,
                        session_id = %session_id,
                        new_status = %status,
                        "[ChatActor] Executing UpdateSessionStatus action"
                    );
                    let _ = self.update_session_status(session_id, status, None).await;
                }
            }
            StateAction::SetActivelyProcessing(value) => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    is_actively_processing = value,
                    "[ChatActor] Executing SetActivelyProcessing action"
                );
                let mut state = self.state.lock().await;
                state.interaction.is_actively_processing = value;
            }
            StateAction::EmitSse(event) => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    event_type = ?event,
                    "[ChatActor] Executing EmitSse action"
                );
                let _ = self.event_tx.send(event);
            }
            StateAction::ResetInactivityTimer => {
                // This is handled by the main loop's select! statement
                tracing::trace!(
                    chat_id = %self.chat_id,
                    "[ChatActor] ResetInactivityTimer action requested"
                );
            }
            StateAction::ShutdownActor => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Executing ShutdownActor action"
                );
                // This will be handled by breaking out of the main loop
                // The state machine will transition to a terminal state first
            }
            StateAction::SaveResponse(response) => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Executing SaveResponse action"
                );
                // This would save the response to the database
                // For now, this is a placeholder for future implementation
                let _ = response;
            }
            StateAction::CancelInteraction => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Executing CancelInteraction action"
                );
                // Cancel the current interaction token
                let token = self.state.lock().await.interaction.current_cancellation_token.clone();
                if let Some(token) = token {
                    tracing::debug!(
                        chat_id = %self.chat_id,
                        "[ChatActor] Cancelled current interaction token"
                    );
                    token.cancel();
                }
            }
            StateAction::SendSuccessResponse => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Executing SendSuccessResponse action"
                );
                // The responder is handled separately in the command processing
                // This action is just a signal that the command succeeded
            }
            StateAction::SendFailureResponse { message } => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    message = %message,
                    "[ChatActor] Executing SendFailureResponse action"
                );
                // The responder is handled separately in the command processing
                // This action is just a signal that the command failed
                let _ = message;
            }
            StateAction::StartProcessing { user_id } => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    user_id = %user_id,
                    "[ChatActor] Executing StartProcessing action - triggering AI interaction"
                );

                // Trigger the AI processing (this is async and may take time)
                let result = self.process_interaction(user_id).await;

                // Update session status based on result
                if let Some(session_id) = self.session_id {
                    let (status, error_msg) = if let Err(ref e) = result {
                        tracing::warn!(
                            chat_id = %self.chat_id,
                            session_id = %session_id,
                            error = %e,
                            "[ChatActor] StartProcessing: Setting status to error (interaction failed)"
                        );
                        (
                            crate::models::agent_session::SessionStatus::Error,
                            Some(format!("AI Engine Error: {}", e))
                        )
                    } else {
                        tracing::debug!(
                            chat_id = %self.chat_id,
                            session_id = %session_id,
                            "[ChatActor] StartProcessing: Setting status to idle (interaction complete)"
                        );
                        (
                            crate::models::agent_session::SessionStatus::Idle,
                            None
                        )
                    };

                    // Update session status
                    match self.update_session_status(session_id, status, error_msg.clone()).await {
                        Ok(_) => {
                            tracing::info!(
                                chat_id = %self.chat_id,
                                session_id = %session_id,
                                "[ChatActor] StartProcessing: Session status updated successfully"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                session_id = %session_id,
                                error = %e,
                                "[ChatActor] StartProcessing: FAILED to update session status"
                            );
                        }
                    }

                    // Clear current task when interaction completes
                    if let Ok(mut conn) = self.pool.acquire().await {
                        let _ = crate::services::agent_sessions::update_session_task(&mut conn, session_id, None, self.user_id).await;
                    }

                    // Send InteractionComplete event to trigger state transition
                    // The actor should now be in Running state, so RunningState will handle this
                    let success = result.is_ok();
                    let error = if let Err(ref e) = result { Some(format!("{}", e)) } else { None };
                    let _ = self.transition_state(
                        ActorEvent::InteractionComplete { success, error },
                        if success { "Interaction completed successfully" } else { "Interaction failed" }
                    ).await;

                    // CRITICAL: Send Done event to frontend to stop blinking cursor
                    if success {
                        let send_result = self.event_tx.send(SseEvent::Done {
                            message: "Turn complete".to_string(),
                        });
                        if let Err(e) = send_result {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                event_type = "Done",
                                error = ?e,
                                receivers = self.event_tx.receiver_count(),
                                "[SSE] FAILED to send Done event - no receivers"
                            );
                        } else {
                            tracing::debug!(
                                chat_id = %self.chat_id,
                                event_type = "Done",
                                receivers = self.event_tx.receiver_count(),
                                "[SSE] SENT Done event successfully"
                            );
                        }
                    }
                }

                if let Err(e) = result {
                    tracing::error!(
                        "[ChatActor] StartProcessing: Error processing interaction for chat {}: {:?}",
                        self.chat_id,
                        e
                    );
                    let send_result = self.event_tx.send(SseEvent::Error {
                        message: format!("AI Engine Error: {}", e),
                    });
                    if let Err(e) = send_result {
                        tracing::error!(
                            chat_id = %self.chat_id,
                            event_type = "Error",
                            error = ?e,
                            receivers = self.event_tx.receiver_count(),
                            "[SSE] FAILED to send error event - no receivers"
                        );
                    }
                }

                // Mark as done processing - actor can now be idle
                {
                    let mut state = self.state.lock().await;
                    state.interaction.is_actively_processing = false;
                }
            }
        }
        Ok(())
    }

    /// Execute multiple state actions in sequence.
    async fn execute_state_actions(&mut self, actions: Vec<StateAction>) -> Result<()> {
        for action in actions {
            self.execute_state_action(action).await?;
        }
        Ok(())
    }

    /// Convert an AgentCommand to an ActorEvent.
    ///
    /// This bridges the existing command system with the new state machine.
    fn command_to_event(&self, command: &AgentCommand) -> Option<ActorEvent> {
        match command {
            AgentCommand::ProcessInteraction { user_id } => {
                Some(ActorEvent::ProcessInteraction { user_id: *user_id })
            }
            AgentCommand::Pause { .. } => {
                Some(ActorEvent::Pause { reason: None })
            }
            AgentCommand::Cancel { reason, .. } => {
                Some(ActorEvent::Cancel { reason: reason.clone() })
            }
            AgentCommand::Ping => {
                Some(ActorEvent::Ping)
            }
            AgentCommand::Shutdown => {
                Some(ActorEvent::Shutdown)
            }
        }
    }

    /// Create a state context for state handlers.
    fn create_state_context(&self) -> StateContext<'_, '_> {
        StateContext {
            chat_id: self.chat_id,
            workspace_id: self.workspace_id,
            user_id: self.user_id,
            pool: self.pool.clone(),
            storage: self.storage.clone(),
            event_tx: self.event_tx.clone(),
            default_persona: self.default_persona.clone(),
            default_context_token_limit: self.default_context_token_limit,
            shared_state: Some(&self.shared_state),
            responder: None,
        }
    }

    /// Create a state context with a responder for Pause/Cancel commands.
    fn create_state_context_with_responder<'b>(
        &self,
        responder: &'b Arc<Mutex<Option<oneshot::Sender<crate::error::Result<bool>>>>>,
    ) -> StateContext<'_, 'b> {
        StateContext {
            chat_id: self.chat_id,
            workspace_id: self.workspace_id,
            user_id: self.user_id,
            pool: self.pool.clone(),
            storage: self.storage.clone(),
            event_tx: self.event_tx.clone(),
            default_persona: self.default_persona.clone(),
            default_context_token_limit: self.default_context_token_limit,
            shared_state: Some(&self.shared_state),
            responder: Some(responder),
        }
    }

    /// Process a command through the state handler system.
    ///
    /// Returns true if the command was fully handled by the state system,
    /// false if it needs to be processed by the legacy code path.
    #[instrument(skip(self, command), fields(chat_id = %self.chat_id))]
    async fn process_command_via_state_handler(
        &mut self,
        command: &AgentCommand,
    ) -> Result<bool> {
        // Extract responder from Pause/Cancel commands
        let responder = match command {
            AgentCommand::Pause { responder, .. } => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Pause: responder extracted"
                );
                Some(responder)
            }
            AgentCommand::Cancel { responder, reason: _ } => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Cancel: responder extracted"
                );
                Some(responder)
            }
            _ => None,
        };

        // Convert command to event
        let event = match self.command_to_event(command) {
            Some(e) => e,
            None => {
                // Command doesn't map to an event, use legacy path
                return Ok(false);
            }
        };

        let current_state = self.current_state();

        // Clone event for use after handle_event (which moves it)
        let event_for_log = event.clone();
        let event_for_transition = event.clone();

        // Get handler before creating mutable context
        let handler = self.state_handlers.get_handler(current_state);

        // Create context with or without responder
        let mut ctx = if let Some(ref responder_arc) = responder {
            self.create_state_context_with_responder(responder_arc)
        } else {
            self.create_state_context()
        };

        // Call the state handler
        let result = handler.handle_event(event, &mut ctx);

        match result {
            Ok(event_result) => {
                tracing::debug!(
                    new_state = ?event_result.new_state,
                    actions_count = event_result.actions.len(),
                    "[ChatActor] State handler result"
                );
                // Remember if there were actions before moving
                let has_actions = !event_result.actions.is_empty();
                let has_state_change = event_result.new_state.is_some();

                // Separate response actions from other actions
                let mut response_actions = Vec::new();
                let mut other_actions = Vec::new();
                for action in event_result.actions {
                    if matches!(
                        action,
                        StateAction::SendSuccessResponse | StateAction::SendFailureResponse { .. }
                    ) {
                        response_actions.push(action);
                    } else {
                        other_actions.push(action);
                    }
                }

                // IMPORTANT: State transition MUST happen BEFORE executing actions
                // This ensures that actions like StartProcessing can send events
                // (like InteractionComplete) that will be handled by the NEW state
                if let Some(new_state) = event_result.new_state {
                    tracing::info!(
                        from = %current_state,
                        to = %new_state,
                        "[ChatActor] Transitioning state BEFORE executing actions"
                    );
                    let _ = self.transition_state(
                        event_for_transition,
                        &format!("State handler: {:?} → {:?}", current_state, new_state)
                    ).await;
                }

                // Execute non-response actions AFTER state transition
                self.execute_state_actions(other_actions).await?;

                // Handle response actions by sending to the responder
                // This MUST happen BEFORE checking for terminal state, so responses are sent
                if let Some(responder_arc) = responder {
                    tracing::info!(
                        chat_id = %self.chat_id,
                        response_actions_count = response_actions.len(),
                        "[ChatActor] Processing response actions"
                    );
                    if let Some(sender_option) = responder_arc.lock().await.take() {
                        tracing::info!(
                            chat_id = %self.chat_id,
                            "[ChatActor] Sending response"
                        );
                        let send_result = if response_actions.iter().any(|a| matches!(a, StateAction::SendSuccessResponse)) {
                            // Success - send Ok(true)
                            sender_option.send(Ok(true))
                        } else if let Some(fail_action) = response_actions.iter().find(|a| matches!(a, StateAction::SendFailureResponse { .. })) {
                            // Failure - send error message
                            if let StateAction::SendFailureResponse { message } = fail_action {
                                sender_option.send(Err(crate::error::Error::Internal(message.clone())))
                            } else {
                                unreachable!()
                            }
                        } else {
                            // No explicit response action, but command was processed successfully
                            sender_option.send(Ok(true))
                        };

                        match &send_result {
                            Ok(_) => tracing::info!(chat_id = %self.chat_id, "[ChatActor] Response sent successfully"),
                            Err(_) => tracing::warn!(chat_id = %self.chat_id, "[ChatActor] Failed to send response"),
                        }
                    } else {
                        tracing::warn!(
                            chat_id = %self.chat_id,
                            "[ChatActor] Responder sender was None (already taken?)"
                        );
                    }
                }

                // NOW check if we should break out of the loop (terminal state)
                // Response has been sent, so we can safely exit
                if let Some(new_state) = event_result.new_state {
                    if new_state.is_terminal() {
                        return Ok(true);
                    }
                }

                // Emit any SSE events
                for sse_event in event_result.emit_sse {
                    let _ = self.event_tx.send(sse_event);
                }

                // Return true if the handler actually did something (state change or actions)
                // Return false to fall through to legacy path
                let handled = has_state_change || has_actions;
                tracing::debug!(
                    handled,
                    has_state_change,
                    has_actions,
                    "[ChatActor] State handler 'handled' result (false=fall through to legacy)"
                );
                Ok(handled)
            }
            Err(e) => {
                // Send failure response to responder if present
                if let Some(responder_arc) = responder {
                    if let Some(sender_option) = responder_arc.lock().await.take() {
                        let error_msg = format!("State handler failed: {}", e);
                        let _ = sender_option.send(Err(crate::error::Error::Internal(error_msg)));
                    }
                }

                warn!(
                    current_state = %current_state,
                    event = ?event_for_log,
                    error = ?e,
                    "[ChatActor] State handler failed, falling back to legacy path"
                );
                Ok(false)
            }
        }
    }
}
