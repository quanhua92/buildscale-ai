use crate::models::chat::{ChatMessageMetadata, ChatMessageRole, NewChatMessage, DEFAULT_CHAT_MODEL};
use crate::models::sse::SseEvent;
use crate::queries;
use crate::services::chat::registry::{AgentCommand, AgentHandle, AgentRegistry};
use crate::services::chat::rig_engine::RigService;
use crate::services::chat::ChatService;
use crate::services::storage::FileStorageService;
use crate::providers::Agent;
use crate::DbPool;
use futures::StreamExt;
use rig::streaming::StreamingChat;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Consolidated state for ChatActor to reduce lock contention
/// All state that was previously in separate Arc<Mutex<>> fields is now grouped logically
struct ChatActorState {
    /// Agent State Management (accessed together in get_or_create_agent)
    agent_state: AgentState,
    /// Tool Tracking (always accessed in pairs)
    tool_tracking: ToolTracking,
    /// Interaction Lifecycle (independent access)
    interaction: InteractionState,
    /// Current reasoning session tracking (for audit trail)
    current_reasoning_id: Option<String>,
    /// Buffer for reasoning chunks (aggregated before DB persistence)
    reasoning_buffer: Vec<String>,
}

/// Agent cache and validation state
/// These fields are accessed together when checking/creating agents
struct AgentState {
    /// Cached agent with preserved chat_history
    /// Contains reasoning items for GPT-5 multi-turn conversations (for OpenAI)
    /// Wraps both OpenAI and OpenRouter agents in our unified Agent enum
    cached_agent: Option<Agent>,
    /// Track model name to detect when to recreate agent
    current_model_name: Option<String>,
    /// Track user_id to detect when to recreate agent
    current_user_id: Option<Uuid>,
    /// Track mode to detect when to recreate agent (mode changes require new ToolConfig)
    current_mode: Option<String>,
}

impl AgentState {
    /// Check if the cached agent can be reused for the given session and user.
    /// Agent can be reused only if all criteria match: model, user_id, and mode.
    fn can_reuse(&self, session: &crate::models::chat::ChatSession, user_id: Uuid) -> bool {
        self.cached_agent.is_some()
            && self.current_model_name.as_ref() == Some(&session.agent_config.model)
            && self.current_user_id.as_ref() == Some(&user_id)
            && self.current_mode.as_ref() == Some(&session.agent_config.mode)
    }
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
            agent_state: AgentState {
                cached_agent: None,
                current_model_name: None,
                current_user_id: None,
                current_mode: None,
            },
            tool_tracking: ToolTracking {
                current_tool_name: None,
                current_tool_args: None,
            },
            interaction: InteractionState {
                current_cancellation_token: None,
                current_model: None,
            },
            current_reasoning_id: None,
            reasoning_buffer: Vec::new(),
        }
    }
}

pub struct ChatActor {
    chat_id: Uuid,
    workspace_id: Uuid,
    pool: DbPool,
    rig_service: Arc<RigService>,
    storage: Arc<FileStorageService>,
    registry: Arc<AgentRegistry>,
    command_rx: mpsc::Receiver<AgentCommand>,
    event_tx: broadcast::Sender<SseEvent>,
    default_persona: String,
    default_context_token_limit: usize,
    inactivity_timeout: std::time::Duration,
    /// Consolidated state - single lock for all actor state
    /// Reduces lock contention and eliminates deadlock risk
    state: Arc<Mutex<ChatActorState>>,
}

pub struct ChatActorArgs {
    pub chat_id: Uuid,
    pub workspace_id: Uuid,
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

        let actor = Self {
            chat_id: args.chat_id,
            workspace_id: args.workspace_id,
            pool: args.pool,
            rig_service: args.rig_service,
            storage: args.storage,
            registry: args.registry,
            command_rx,
            event_tx: args.event_tx,
            default_persona: args.default_persona,
            default_context_token_limit: args.default_context_token_limit,
            inactivity_timeout: args.inactivity_timeout,
            state: Arc::new(Mutex::new(ChatActorState::default())),
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
        tracing::info!("[ChatActor] Started for chat {}", self.chat_id);

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
                    tracing::info!("[ChatActor] Shutting down due to inactivity for chat {}", self.chat_id);
                    break;
                }
                command = self.command_rx.recv() => {
                    if let Some(cmd) = command {
                        // Reset inactivity timeout on any command
                        inactivity_timeout.as_mut().reset(tokio::time::Instant::now() + inactivity_timeout_duration);

                        match cmd {
                            AgentCommand::ProcessInteraction { user_id } => {
                                if let Err(e) = self.process_interaction(user_id).await {

                                    tracing::error!(
                                        "[ChatActor] Error processing interaction for chat {}: {:?}",
                                        self.chat_id,
                                        e
                                    );
                                    let _ = self.event_tx.send(SseEvent::Error {
                                        message: format!("AI Engine Error: {}", e),
                                    });
                                }

                                // Reset inactivity timeout AGAIN after work completes
                                // This ensures the idle period starts from the end of the interaction.
                                inactivity_timeout.as_mut().reset(tokio::time::Instant::now() + inactivity_timeout_duration);
                            }
                            AgentCommand::Ping => {
                                tracing::debug!("ChatActor received ping for chat {}", self.chat_id);
                            }
                            AgentCommand::Cancel { reason, responder } => {
                                tracing::info!(
                                    "[ChatActor] Cancel requested for chat {} (reason: {})",
                                    self.chat_id,
                                    reason
                                );

                                // Trigger cancellation of current interaction's token
                                let token = self.state.lock().await.interaction.current_cancellation_token.clone();
                                if let Some(token) = token {
                                    token.cancel();
                                }

                                // Send acknowledgment
                                if let Some(responder) = responder.lock().await.take() {
                                    let _ = responder.send(Ok(true));
                                }

                                // Don't break the loop - actor continues running
                            }
                            AgentCommand::Shutdown => {
                                tracing::info!("[ChatActor] Shutting down for chat {}", self.chat_id);
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }

    async fn process_interaction(&self, user_id: Uuid) -> crate::error::Result<()> {
        tracing::info!("[ChatActor] Processing interaction for chat {}", self.chat_id);

        // Create a new cancellation token for this interaction
        let cancellation_token = CancellationToken::new();
        self.state.lock().await.interaction.current_cancellation_token = Some(cancellation_token.clone());

        let mut conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

        // 1. Build structured context with persona, history, and attachments
        let context = ChatService::build_context(
            &mut conn,
            &self.storage,
            self.workspace_id,
            self.chat_id,
            &self.default_persona,
            self.default_context_token_limit,
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

        // Process stream based on provider type
        let full_response = match &agent {
            Agent::OpenAI(openai_agent) => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "Calling OpenAI agent.stream_chat"
                );
                let stream = openai_agent.stream_chat(&prompt, history).await;
                tracing::info!(
                    chat_id = %self.chat_id,
                    "Stream created (OpenAI), entering response loop"
                );
                self.process_agent_stream(stream, &cancellation_token, &mut conn, &session, &mut item_count).await?
            }
            Agent::OpenRouter(openrouter_agent) => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "Calling OpenRouter agent.stream_chat"
                );
                let stream = openrouter_agent.stream_chat(&prompt, history).await;
                tracing::info!(
                    chat_id = %self.chat_id,
                    "Stream created (OpenRouter), entering response loop"
                );
                self.process_agent_stream(stream, &cancellation_token, &mut conn, &session, &mut item_count).await?
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

        let _ = self.event_tx.send(SseEvent::Done {
            message: "Turn complete".to_string(),
        });

        tracing::info!("[ChatActor] Interaction turn complete for chat {}", self.chat_id);

        // Clear the cancellation token for this interaction
        self.state.lock().await.interaction.current_cancellation_token = None;

        // Clear reasoning ID for next interaction
        self.state.lock().await.current_reasoning_id = None;

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

        // 5. Clear agent cache to ensure fresh state after cancellation
        self.state.lock().await.agent_state.cached_agent = None;

        // 6. Clear reasoning ID for next interaction
        self.state.lock().await.current_reasoning_id = None;

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
                            text_preview = %format!("{}...", &text.text[..text.text.len().min(50)]),
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
                        if let Err(e) = self.event_tx.send(SseEvent::Thought {
                            agent_id: None,
                            text: reasoning.clone(),
                        }) {
                            tracing::error!("[ChatActor] [SSE] Failed to send Thought event: {:?}", e);
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
                                if let Err(e) = self.event_tx.send(SseEvent::Thought {
                                    agent_id: None,
                                    text: part.clone(),
                                }) {
                                    tracing::error!("[ChatActor] [SSE] Failed to send Thought event: {:?}", e);
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
                                format!("{}...\n[Arguments truncated, see metadata for full details]", &args_str[..500])
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

                        if let Err(e) = self.event_tx.send(SseEvent::Call {
                            tool: tool_call.function.name,
                            path,
                            args: tool_call.function.arguments,
                        }) {
                            tracing::error!("[ChatActor] [SSE] Failed to send Call event: {:?}", e);
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
                        let has_error_heuristic = |s: &str| {
                            s.starts_with("Error:")
                                || s.starts_with("error:")
                                || s.contains("ToolCallError")
                                || (s.contains("Tool error") && s.contains("failed"))
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

                                         // Emit mode_changed event to frontend
                                         let _ = self.event_tx.send(SseEvent::ModeChanged {
                                             mode: "build".to_string(),
                                             plan_file: if plan_file.is_empty() { None } else { Some(plan_file.to_string()) },
                                         });

                                         // Clear agent cache to force new agent with build mode
                                         self.state.lock().await.agent_state.cached_agent = None;
                                     }
                                 }
                             }
                         }

                         tracing::info!(
                             "[ChatActor] [Rig] Tool execution finished for chat {} (success: {}). Output: {}",
                             self.chat_id,
                             success,
                             if output.len() > 100 {
                                 let mut end = 100;
                                 while end > 0 && !output.is_char_boundary(end) {
                                     end -= 1;
                                 }
                                 format!("{}...", &output[..end])
                             } else {
                                 output.clone()
                             }
                         );
                          if let Err(e) = self.event_tx.send(SseEvent::Observation { output: normalized_output.clone(), success }) {
                              tracing::error!("[ChatActor] [SSE] Failed to send Observation event: {:?}", e);
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
    ) -> crate::error::Result<String>
    where
        S: futures::Stream<Item = Result<rig::agent::MultiTurnStreamItem<M>, E>> + Unpin,
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
            let item = tokio::select! {
                _ = cancellation_token.cancelled() => {
                    tracing::info!("[ChatActor] Cancelled during streaming for chat {}", self.chat_id);
                    self.handle_cancellation(conn, full_response.clone(), "user_cancelled").await?;
                    return Err(crate::error::Error::Internal("Chat cancelled by user".to_string()));
                },
                item = stream.next() => {
                    match &item {
                        Some(Ok(_)) => tracing::debug!(chat_id = %self.chat_id, item_num = *item_count, "Received stream item"),
                        Some(Err(e)) => tracing::error!(chat_id = %self.chat_id, error = %e, "Stream error"),
                        None => tracing::info!(chat_id = %self.chat_id, "Stream ended (None)"),
                    }
                    item
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
        let mut state = self.state.lock().await;

        // Check if we can reuse the cached agent
        if state.agent_state.can_reuse(session, user_id) {
            Ok(state.agent_state.cached_agent.as_ref().unwrap().clone())
        } else {
            tracing::info!(
                "[ChatActor] Creating new agent for chat {} (model: {})",
                self.chat_id, session.agent_config.model
            );

            // Create new agent
            let agent = self.rig_service.create_agent(
                self.pool.clone(),
                self.storage.clone(),
                self.workspace_id,
                self.chat_id,
                user_id,
                session,
                ai_config,
            ).await?;

            // Update cache - single atomic update
            state.agent_state.cached_agent = Some(agent.clone());
            state.agent_state.current_model_name = Some(session.agent_config.model.clone());
            state.agent_state.current_user_id = Some(user_id);
            state.agent_state.current_mode = Some(session.agent_config.mode.clone());

            Ok(agent)
        }
    }
}
