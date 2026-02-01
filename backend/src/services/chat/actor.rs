use crate::models::chat::{ChatMessageRole, NewChatMessage, DEFAULT_CHAT_MODEL};
use crate::models::sse::SseEvent;
use crate::queries;
use crate::services::chat::registry::{AgentCommand, AgentHandle};
use crate::services::chat::rig_engine::RigService;
use crate::services::chat::ChatService;
use crate::services::storage::FileStorageService;
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
    storage: Arc<FileStorageService>,
    command_rx: mpsc::Receiver<AgentCommand>,
    event_tx: broadcast::Sender<SseEvent>,
    default_persona: String,
    default_context_token_limit: usize,
    inactivity_timeout: std::time::Duration,
    current_cancellation_token: Arc<Mutex<Option<CancellationToken>>>,
    current_model: Arc<Mutex<Option<String>>>, // Track current model for cancellation metadata
    /// Cached Rig agent with preserved chat_history
    /// Contains reasoning items for GPT-5 multi-turn conversations
    cached_agent: Arc<Mutex<Option<rig::agent::Agent<rig::providers::openai::responses_api::ResponsesCompletionModel>>>>,
    /// Track model name to detect when to recreate agent
    current_model_name: Arc<Mutex<Option<String>>>,
    /// Track user_id to detect when to recreate agent
    current_user_id: Arc<Mutex<Option<Uuid>>>,
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
            current_cancellation_token: Arc::new(Mutex::new(None)),
            current_model: Arc::new(Mutex::new(None)),
            cached_agent: Arc::new(Mutex::new(None)),
            current_model_name: Arc::new(Mutex::new(None)),
            current_user_id: Arc::new(Mutex::new(None)),
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
        *self.current_model.lock().await = Some(session.agent_config.model.clone());

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
        let mut stream = agent
            .stream_chat(&prompt, history)
            .await;

        tracing::info!(
            chat_id = %self.chat_id,
            "Stream created, entering response loop"
        );

        let mut full_response = String::new();
        let mut has_started_responding = false;
        let mut item_count = 0usize;

        tracing::debug!(
            chat_id = %self.chat_id,
            "Starting stream loop"
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
                    break; // Stream finished naturally
                }
            };

            // Track stream items for debugging
            item_count += 1;

            // Debug: Log the item type before matching
            tracing::debug!(
                chat_id = %self.chat_id,
                item_num = item_count,
                is_err = item.is_err(),
                is_ok = item.is_ok(),
                "Processing stream item"
            );

            match item {
                Err(e) => {
                    tracing::error!(
                        chat_id = %self.chat_id,
                        error = %e,
                        item_num = item_count,
                        "Stream item error, wrapping as Internal error"
                    );
                    return Err(crate::error::Error::Internal(format!("Streaming error: {}", e)));
                }
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
                            // Only send non-empty reasoning parts to frontend
                            for part in &thought.reasoning {
                                if !part.trim().is_empty() {
                                    let _ = self.event_tx.send(SseEvent::Thought {
                                        agent_id: None,
                                        text: part.clone(),
                                    });
                                }
                            }
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
                                             if let Ok(mut conn) = self.pool.acquire().await.map_err(crate::error::Error::Sqlx) {
                                                 if let Err(e) = ChatService::update_chat_metadata(
                                                     &mut conn,
                                                     &self.storage,
                                                     self.workspace_id,
                                                     self.chat_id,
                                                     mode.to_string(),
                                                     if plan_file.is_empty() { None } else { Some(plan_file.to_string()) },
                                                 ).await {
                                                     tracing::error!(
                                                         "[ChatActor] Failed to update chat metadata: {:?}",
                                                         e
                                                     );
                                                 } else {
                                                     tracing::info!(
                                                         "[ChatActor] Successfully updated chat {} metadata: mode={}, plan_file={}",
                                                         self.chat_id,
                                                         mode,
                                                         plan_file
                                                     );
                                                 }
                                             }

                                             // Emit mode_changed event to frontend
                                             let _ = self.event_tx.send(SseEvent::ModeChanged {
                                                 mode: "build".to_string(),
                                                 plan_file: if plan_file.is_empty() { None } else { Some(plan_file.to_string()) },
                                             });

                                             // Clear agent cache to force new agent with build mode
                                             *self.cached_agent.lock().await = None;
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
                             let _ = self.event_tx.send(SseEvent::Observation { output, success });
                        }
                    }
                }
                Ok(rig::agent::MultiTurnStreamItem::FinalResponse(final_response)) => {
                    tracing::info!(
                        chat_id = %self.chat_id,
                        response_len = final_response.response().len(),
                        response_text = %final_response.response(),
                        usage = ?final_response.usage(),
                        "Received FinalResponse from stream"
                    );

                    // Send the final response as text if not empty
                    let response_text = final_response.response();
                    if !response_text.is_empty() {
                        if !has_started_responding {
                            tracing::info!("[ChatActor] AI started responding (via FinalResponse) for chat {}", self.chat_id);
                            has_started_responding = true;
                        }
                        full_response.push_str(response_text);
                        let _ = self.event_tx.send(SseEvent::Chunk { text: response_text.to_string() });
                    }
                }
                // Catch-all for future Rig variants (MultiTurnStreamItem is non-exhaustive)
                Ok(other) => {
                    tracing::warn!(
                        chat_id = %self.chat_id,
                        item_num = item_count,
                        "Unhandled stream item variant: {:?}",
                        std::mem::discriminant(&other)
                    );
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

        // Get current model for metadata
        let model = self.current_model.lock().await.clone().unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());

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
        *self.current_cancellation_token.lock().await = None;

        // 5. Clear agent cache to ensure fresh state after cancellation
        *self.cached_agent.lock().await = None;

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

    async fn get_or_create_agent(
        &self,
        user_id: Uuid,
        session: &crate::models::chat::ChatSession,
        ai_config: &crate::config::AiConfig,
    ) -> crate::error::Result<rig::agent::Agent<rig::providers::openai::responses_api::ResponsesCompletionModel>> {
        let mut cached = self.cached_agent.lock().await;
        let current_model_name = self.current_model_name.lock().await;
        let current_user = self.current_user_id.lock().await;

        // Check if we can reuse the cached agent
        let can_reuse = cached.is_some()
            && current_model_name.as_ref() == Some(&session.agent_config.model)
            && current_user.as_ref() == Some(&user_id);

        if can_reuse {
            Ok(cached.as_ref().unwrap().clone())
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

            // Update cache
            *cached = Some(agent.clone());
            drop(current_model_name);
            *self.current_model_name.lock().await = Some(session.agent_config.model.clone());
            drop(current_user);
            *self.current_user_id.lock().await = Some(user_id);

            Ok(agent)
        }
    }
}
