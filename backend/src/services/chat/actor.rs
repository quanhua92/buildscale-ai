use crate::models::chat::{ChatMessageRole, NewChatMessage};
use crate::models::sse::SseEvent;
use crate::queries;
use crate::services::chat::registry::{AgentCommand, AgentHandle};
use crate::services::chat::rig_engine::RigService;
use crate::services::chat::ChatService;
use crate::DbPool;
use futures::StreamExt;
use rig::streaming::StreamingChat;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct ChatActor {
    chat_id: Uuid,
    workspace_id: Uuid,
    pool: DbPool,
    rig_service: Arc<RigService>,
    command_rx: mpsc::Receiver<AgentCommand>,
    event_tx: broadcast::Sender<SseEvent>,
    default_persona: String,
    default_context_token_limit: usize,
    inactivity_timeout: std::time::Duration,
    current_cancellation_token: Arc<Mutex<Option<CancellationToken>>>,
}

pub struct ChatActorArgs {
    pub chat_id: Uuid,
    pub workspace_id: Uuid,
    pub pool: DbPool,
    pub rig_service: Arc<RigService>,
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
            command_rx,
            event_tx: args.event_tx,
            default_persona: args.default_persona,
            default_context_token_limit: args.default_context_token_limit,
            inactivity_timeout: args.inactivity_timeout,
            current_cancellation_token: Arc::new(Mutex::new(None)),
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
                                let token = self.current_cancellation_token.lock().await.clone();
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
        *self.current_cancellation_token.lock().await = Some(cancellation_token.clone());

        let mut conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

        // 1. Build structured context with persona, history, and attachments
        let context = ChatService::build_context(
            &mut conn,
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
                model: "gpt-4o-mini".to_string(),
                temperature: 0.7,
                persona_override: Some(context.persona),
            }
        };

        let session = crate::models::chat::ChatSession {
            file_id: self.chat_id,
            agent_config,
            messages: messages.clone(),
        };

        // 7. Create Rig Agent
        let agent = self
            .rig_service
            .create_agent(self.pool.clone(), self.workspace_id, user_id, &session)
            .await?;

        // 8. Stream from Rig with persona, history, and attachments in prompt
        tracing::info!("[ChatActor] [Rig] Starting AI completion for chat {} (model: {})", self.chat_id, session.agent_config.model);
        let mut stream = agent
            .stream_chat(&prompt, history)
            .await;

        let mut full_response = String::new();
        let mut has_started_responding = false;

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
                item = stream.next() => { item }
            };

            let item = match item {
                Some(i) => i,
                None => break, // Stream finished naturally
            };
            match item {
                Ok(rig::agent::MultiTurnStreamItem::StreamAssistantItem(content)) => {
                    match content {
                        rig::streaming::StreamedAssistantContent::Text(text) => {
                            if !has_started_responding {
                                tracing::info!("[ChatActor] [Rig] AI started streaming text response for chat {}", self.chat_id);
                                has_started_responding = true;
                            }
                            full_response.push_str(&text.text);
                            let _ = self.event_tx.send(SseEvent::Chunk { text: text.text });
                        }
                        rig::streaming::StreamedAssistantContent::Reasoning(thought) => {
                            let text = thought.reasoning.join("\n");
                            tracing::info!("[ChatActor] [Rig] AI thought for chat {}: {}", self.chat_id, text);
                            let _ = self.event_tx.send(SseEvent::Thought {
                                agent_id: None,
                                text,
                            });
                        }
                        rig::streaming::StreamedAssistantContent::ToolCall(tool_call) => {
                            tracing::info!("[ChatActor] [Rig] AI calling tool {} for chat {}", tool_call.function.name, self.chat_id);
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
                Ok(rig::agent::MultiTurnStreamItem::StreamUserItem(content)) => {
                    match content {
                        rig::streaming::StreamedUserContent::ToolResult(result) => {
                            let output = if let Some(rig::completion::message::ToolResultContent::Text(text)) = result.content.iter().next() {
                                text.text.clone()
                            } else {
                                "Tool execution completed".to_string()
                            };

                            // Heuristic: if the output contains "Error:" it's likely a failure
                            let success = !output.to_lowercase().contains("error:");

                            tracing::info!("[ChatActor] [Rig] Tool execution finished for chat {} (success: {})", self.chat_id, success);
                            let _ = self.event_tx.send(SseEvent::Observation { output, success });
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("[ChatActor] [Rig] AI stream encountered an error for chat {}: {:?}", self.chat_id, e);
                    return Err(crate::error::Error::Llm(e.to_string()));
                }
                _ => {}
            }
        }

        // 7. Save Assistant Response
        if !full_response.is_empty() {
            tracing::info!("[ChatActor] Saving AI response to database for chat {}", self.chat_id);
            let mut final_conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;
            queries::chat::insert_chat_message(
                &mut final_conn,
                NewChatMessage {
                    file_id: self.chat_id,
                    workspace_id: self.workspace_id,
                    role: ChatMessageRole::Assistant,
                    content: full_response,
                    metadata: sqlx::types::Json(crate::models::chat::ChatMessageMetadata::default()),
                },
            )
            .await?;
        }

        let _ = self.event_tx.send(SseEvent::Done {
            message: "Turn complete".to_string(),
        });

        tracing::info!("[ChatActor] Interaction turn complete for chat {}", self.chat_id);

        // Clear the cancellation token for this interaction
        *self.current_cancellation_token.lock().await = None;

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
                "[ChatActor] Saving partial response ({} chars) for chat {}",
                partial_response.len(),
                self.chat_id
            );
            self.save_partial_response(conn, partial_response.clone()).await?;
        }

        // 3. Add cancellation marker to chat history for AI awareness
        self.add_cancellation_marker(conn, reason).await?;

        // 4. Clear the cancellation token for this interaction
        *self.current_cancellation_token.lock().await = None;

        Ok(())
    }

    async fn save_partial_response(
        &self,
        conn: &mut sqlx::PgConnection,
        content: String,
    ) -> crate::error::Result<()> {
        queries::chat::insert_chat_message(
            conn,
            NewChatMessage {
                file_id: self.chat_id,
                workspace_id: self.workspace_id,
                role: ChatMessageRole::Assistant,
                content,
                metadata: sqlx::types::Json(crate::models::chat::ChatMessageMetadata::default()),
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

        queries::chat::insert_chat_message(
            conn,
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
}
