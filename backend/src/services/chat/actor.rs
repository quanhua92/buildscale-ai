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
use tokio::sync::{broadcast, mpsc};
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
}

impl ChatActor {
    pub fn spawn(
        chat_id: Uuid,
        workspace_id: Uuid,
        pool: DbPool,
        rig_service: Arc<RigService>,
        default_persona: String,
        default_context_token_limit: usize,
    ) -> AgentHandle {
        let (command_tx, command_rx) = mpsc::channel(32);
        let (event_tx, _) = broadcast::channel(100);

        let actor = Self {
            chat_id,
            workspace_id,
            pool,
            rig_service,
            command_rx,
            event_tx: event_tx.clone(),
            default_persona,
            default_context_token_limit,
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
        tracing::info!("ChatActor started for chat {}", self.chat_id);

        // Periodic heartbeat ping (every 10 seconds)
        let mut heartbeat_interval = tokio::time::interval(std::time::Duration::from_secs(10));

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    let _ = self.event_tx.send(SseEvent::Ping);
                }
                command = self.command_rx.recv() => {
                    if let Some(cmd) = command {
                        match cmd {
                            AgentCommand::ProcessInteraction { user_id } => {
                                // Send initial thought to signal activity immediately
                                let _ = self.event_tx.send(SseEvent::Thought {
                                    agent_id: None,
                                    text: "Initializing context and connecting to AI brain...".to_string(),
                                });

                                if let Err(e) = self.process_interaction(user_id).await {
                                    tracing::error!(
                                        "Error processing interaction for chat {}: {:?}",
                                        self.chat_id,
                                        e
                                    );
                                    let _ = self.event_tx.send(SseEvent::Error {
                                        message: format!("AI Engine Error: {}", e),
                                    });
                                }
                            }
                            AgentCommand::Shutdown => {
                                tracing::info!("ChatActor shutting down for chat {}", self.chat_id);
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
        tracing::debug!(chat_id = %self.chat_id, "Processing interaction");

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
        tracing::info!(chat_id = %self.chat_id, model = %session.agent_config.model, "Requesting AI completion");
        let mut stream = agent
            .stream_chat(&prompt, history)
            .await;

        let mut full_response = String::new();
        let mut has_started_responding = false;

        while let Some(item) = stream.next().await {
            match item {
                Ok(rig::agent::MultiTurnStreamItem::StreamAssistantItem(content)) => {
                    match content {
                        rig::streaming::StreamedAssistantContent::Text(text) => {
                            if !has_started_responding {
                                tracing::debug!(chat_id = %self.chat_id, "AI started streaming text response");
                                has_started_responding = true;
                            }
                            full_response.push_str(&text.text);
                            let _ = self.event_tx.send(SseEvent::Chunk { text: text.text });
                        }
                        rig::streaming::StreamedAssistantContent::Reasoning(thought) => {
                            let text = thought.reasoning.join("\n");
                            tracing::debug!(chat_id = %self.chat_id, thought = %text, "AI thought");
                            let _ = self.event_tx.send(SseEvent::Thought {
                                agent_id: None,
                                text,
                            });
                        }
                        rig::streaming::StreamedAssistantContent::ToolCall(tool_call) => {
                            tracing::info!(chat_id = %self.chat_id, tool = %tool_call.function.name, "AI calling tool");
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
                            tracing::info!(chat_id = %self.chat_id, "Tool execution finished");
                            let _ = self.event_tx.send(SseEvent::Observation { output });
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(chat_id = %self.chat_id, error = ?e, "AI stream encountered an error");
                    return Err(crate::error::Error::Llm(e.to_string()));
                }
                _ => {}
            }
        }

        // 7. Save Assistant Response
        if !full_response.is_empty() {
            tracing::info!(chat_id = %self.chat_id, "Saving AI response to database");
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

        Ok(())
    }
}
