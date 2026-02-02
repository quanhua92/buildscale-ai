use crate::models::chat::{ChatMessageRole, NewChatMessage, DEFAULT_CHAT_MODEL};
use crate::models::sse::SseEvent;
use crate::queries;
use crate::services::chat::registry::{AgentCommand, AgentHandle};
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
    /// Accumulated State (independent access)
    tool_actions_log: Vec<String>,
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
            tool_actions_log: Vec::new(),
        }
    }
}

pub struct ChatActor {
    chat_id: Uuid,
    workspace_id: Uuid,
    pool: DbPool,
    rig_service: Arc<RigService>,
    storage: Arc<FileStorageService>,
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

        // 3. Format file attachments from ContextManager
        // The ContextManager has already optimized and sorted attachments by priority
        let attachments_context = if !context.attachment_manager.map.is_empty() {
            context.attachment_manager.render()
        } else {
            String::new()
        };

        // 4. Build full prompt with attachments
        let prompt = format!("{}{}", last_message.content, attachments_context);

        // 5. Convert history to Rig format (exclude last/current message)
        let history = self
            .rig_service
            .convert_history(&context.history.messages);

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

        let mut full_response = String::new();
        let mut has_started_responding = false;
        let mut item_count = 0usize;

        // Process stream based on provider type
        match &agent {
            Agent::OpenAI(openai_agent) => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "Calling OpenAI agent.stream_chat"
                );
                let mut stream = openai_agent.stream_chat(&prompt, history).await;

                tracing::info!(
                    chat_id = %self.chat_id,
                    "Stream created (OpenAI), entering response loop"
                );

                loop {
                    // Check for cancellation before each stream iteration
                    if cancellation_token.is_cancelled() {
                        tracing::info!("[ChatActor] Cancelled during streaming for chat {}", self.chat_id);
                        return self.handle_cancellation(&mut conn, full_response, "user_cancelled").await;
                    }

                    // Use tokio::select! to allow cancellation during stream.next()
                    let item = tokio::select! {
                        _ = cancellation_token.cancelled() => {
                            tracing::info!("[ChatActor] Cancelled during streaming for chat {}", self.chat_id);
                            return self.handle_cancellation(&mut conn, full_response, "user_cancelled").await;
                        },
                        item = stream.next() => {
                            match &item {
                                Some(Ok(_)) => tracing::debug!(chat_id = %self.chat_id, item_num = item_count, "Received stream item"),
                                Some(Err(e)) => tracing::error!(chat_id = %self.chat_id, error = %e, "Stream error"),
                                None => tracing::info!(chat_id = %self.chat_id, "Stream ended (None)"),
                            }
                            item
                        }
                    };

                    let item = match item {
                        Some(i) => i,
                        None => {
                            tracing::info!(chat_id = %self.chat_id, items_received = item_count, "Stream finished naturally");
                            break;
                        }
                    };

                    item_count += 1;

                    match item {
                        Err(e) => {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                error = %e,
                                item_num = item_count,
                                "Stream item error"
                            );
                            return Err(crate::error::Error::Internal(format!("Streaming error: {}", e)));
                        }
                        Ok(stream_item) => {
                            if let Err(e) = self.process_stream_item(
                                stream_item,
                                &mut full_response,
                                &mut has_started_responding,
                                item_count,
                                &mut conn,
                                &session,
                                &cancellation_token,
                            ).await {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            Agent::OpenRouter(openrouter_agent) => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "Calling OpenRouter agent.stream_chat"
                );
                let mut stream = openrouter_agent.stream_chat(&prompt, history).await;

                tracing::info!(
                    chat_id = %self.chat_id,
                    "Stream created (OpenRouter), entering response loop"
                );

                loop {
                    // Check for cancellation before each stream iteration
                    if cancellation_token.is_cancelled() {
                        tracing::info!("[ChatActor] Cancelled during streaming for chat {}", self.chat_id);
                        return self.handle_cancellation(&mut conn, full_response, "user_cancelled").await;
                    }

                    // Use tokio::select! to allow cancellation during stream.next()
                    let item = tokio::select! {
                        _ = cancellation_token.cancelled() => {
                            tracing::info!("[ChatActor] Cancelled during streaming for chat {}", self.chat_id);
                            return self.handle_cancellation(&mut conn, full_response, "user_cancelled").await;
                        },
                        item = stream.next() => {
                            match &item {
                                Some(Ok(_)) => tracing::debug!(chat_id = %self.chat_id, item_num = item_count, "Received stream item"),
                                Some(Err(e)) => tracing::error!(chat_id = %self.chat_id, error = %e, "Stream error"),
                                None => tracing::info!(chat_id = %self.chat_id, "Stream ended (None)"),
                            }
                            item
                        }
                    };

                    let item = match item {
                        Some(i) => i,
                        None => {
                            tracing::info!(chat_id = %self.chat_id, items_received = item_count, "Stream finished naturally");
                            break;
                        }
                    };

                    item_count += 1;

                    match item {
                        Err(e) => {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                error = %e,
                                item_num = item_count,
                                "Stream item error"
                            );
                            return Err(crate::error::Error::Internal(format!("Streaming error: {}", e)));
                        }
                        Ok(stream_item) => {
                            if let Err(e) = self.process_stream_item(
                                stream_item,
                                &mut full_response,
                                &mut has_started_responding,
                                item_count,
                                &mut conn,
                                &session,
                                &cancellation_token,
                            ).await {
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }
        // Check if stream completed without any items (possible API access issue)
        if item_count == 0 {
            tracing::warn!(
                "[ChatActor] [Rig] Stream completed with 0 items for chat {} (model: {}). This may indicate an API access issue or invalid model name.",
                self.chat_id,
                session.agent_config.model
            );
        }

        // Save tool actions log before AI response
        let log = self.state.lock().await.tool_actions_log.clone();
        if !log.is_empty() {
            tracing::info!(
                "[ChatActor] Saving {} tool actions log before AI response for chat {}",
                log.len(),
                self.chat_id
            );
            let combined_log = log.join("\n");

            // Send tool action log via SSE for real-time display
            let _ = self.event_tx.send(SseEvent::Chunk { text: combined_log.clone() });

            let mut log_conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;
            ChatService::save_message(
                &mut log_conn,
                &self.storage,
                self.workspace_id,
                NewChatMessage {
                    file_id: self.chat_id,
                    workspace_id: self.workspace_id,
                    role: ChatMessageRole::Assistant,
                    content: combined_log,
                    metadata: sqlx::types::Json(crate::models::chat::ChatMessageMetadata {
                        model: Some(session.agent_config.model.clone()),
                        ..Default::default()
                    }),
                },
            )
            .await?;
        }

        // 7. Save Assistant Response
        if !full_response.is_empty() {
            tracing::info!("[ChatActor] Saving AI response to database for chat {} (model: {})", self.chat_id, session.agent_config.model);
            let mut final_conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;
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

        // Clear tool actions log for next interaction
        self.state.lock().await.tool_actions_log.clear();

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
        _conn: &mut sqlx::PgConnection,
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
                        tracing::info!(
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
                        if let Err(e) = self.event_tx.send(SseEvent::Chunk { text: text.text.clone() }) {
                            tracing::error!(
                                chat_id = %self.chat_id,
                                error = %e,
                                "Failed to send Chunk event to frontend"
                            );
                        } else {
                            tracing::debug!(
                                chat_id = %self.chat_id,
                                text_len = text.text.len(),
                                "Successfully sent Chunk event to frontend"
                            );
                        }
                    }
                    rig::streaming::StreamedAssistantContent::Reasoning(thought) => {
                        tracing::info!(
                            chat_id = %self.chat_id,
                            reasoning_parts = thought.reasoning.len(),
                            "[ChatActor] [Rig] Received Reasoning tokens"
                        );
                        // Only send non-empty reasoning parts to frontend
                        for part in &thought.reasoning {
                            if !part.trim().is_empty() {
                                tracing::debug!(
                                    chat_id = %self.chat_id,
                                    reasoning_len = part.len(),
                                    "[ChatActor] [Rig] Sending reasoning part to frontend"
                                );
                                let _ = self.event_tx.send(SseEvent::Thought {
                                    agent_id: None,
                                    text: part.clone(),
                                });
                            }
                        }
                    }
                    rig::streaming::StreamedAssistantContent::ToolCall(tool_call) => {
                        tracing::info!("[ChatActor] [Rig] AI calling tool {} for chat {}", tool_call.function.name, self.chat_id);
                        // Track tool name and arguments for logging when ToolResult arrives
                        {
                            let mut state = self.state.lock().await;
                            state.tool_tracking.current_tool_name = Some(tool_call.function.name.clone());
                            state.tool_tracking.current_tool_args = Some(serde_json::to_value(&tool_call.function.arguments).unwrap_or_default());
                        }
                        let path = tool_call.function.arguments.get("path")
                            .or_else(|| tool_call.function.arguments.get("source"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let _ = self.event_tx.send(SseEvent::Call {
                            tool: tool_call.function.name,
                            path,
                            args: tool_call.function.arguments,
                        });
                    }
                    _ => {}
                }
            }
            rig::agent::MultiTurnStreamItem::StreamUserItem(content) => {
                match content {
                    rig::streaming::StreamedUserContent::ToolResult(result) => {
                        let output = if let Some(rig::completion::message::ToolResultContent::Text(text)) = result.content.iter().next() {
                            text.text.clone()
                        } else {
                            "Tool execution completed".to_string()
                        };

                         // Heuristic: if the output contains "Error:" it's likely a failure
                         let success = !output.to_lowercase().contains("error:");

                         // Check if this is an ask_user tool result with question_pending
                         // Only ask_user returns question_pending status
                         if success {
                             if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&output) {
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
                         let _ = self.event_tx.send(SseEvent::Observation { output: output.clone(), success });

                         // Log successful file modification tools
                         if success {
                             let (name_opt, args_opt) = {
                                 let state = self.state.lock().await;
                                 (state.tool_tracking.current_tool_name.clone(),
                                  state.tool_tracking.current_tool_args.clone())
                             };

                             if let Some(tname) = name_opt {
                                 if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&output) {
                                     let args_ref = args_opt.as_ref();
                                     if let Some(log_msg) = ChatService::format_tool_action(
                                         &tname,
                                         &result_json,
                                         args_ref,
                                     ) {
                                         tracing::info!(
                                             "[ChatActor] Adding tool action to log: {}",
                                             log_msg
                                         );
                                         self.state.lock().await.tool_actions_log.push(log_msg);
                                     }
                                 }
                             }
                         }

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
                tracing::info!(
                    chat_id = %self.chat_id,
                    response_len = final_response.response().len(),
                    response_text = %final_response.response(),
                    usage = ?final_response.usage(),
                    "Received FinalResponse from stream"
                );

                // Store final response for database save, but DON'T send as SSE chunk
                // The response has already been streamed via Text chunks above
                let response_text = final_response.response();
                if !response_text.is_empty() {
                    if !*has_started_responding {
                        tracing::info!("[ChatActor] AI started responding (via FinalResponse) for chat {}", self.chat_id);
                        *has_started_responding = true;
                    }
                    // Only store for database, don't send as SSE chunk (already streamed)
                    full_response.push_str(response_text);
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
