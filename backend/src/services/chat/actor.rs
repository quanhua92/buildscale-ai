use crate::models::chat::{ChatMessageRole, NewChatMessage};
use crate::models::sse::SseEvent;
use crate::queries;
use crate::services::chat::registry::{AgentCommand, AgentHandle};
use crate::services::chat::rig_engine::RigService;
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
}

impl ChatActor {
    pub fn spawn(
        chat_id: Uuid,
        workspace_id: Uuid,
        pool: DbPool,
        rig_service: Arc<RigService>,
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

        while let Some(command) = self.command_rx.recv().await {
            match command {
                AgentCommand::ProcessInteraction { user_id, content } => {
                    if let Err(e) = self.process_interaction(user_id, content).await {
                        tracing::error!(
                            "Error processing interaction for chat {}: {}",
                            self.chat_id,
                            e
                        );
                        let _ = self.event_tx.send(SseEvent::Error {
                            message: e.to_string(),
                        });
                    }
                }
                AgentCommand::Shutdown => {
                    tracing::info!("ChatActor shutting down for chat {}", self.chat_id);
                    break;
                }
            }
        }
    }

    async fn process_interaction(&self, user_id: Uuid, _content: String) -> crate::error::Result<()> {
        let mut conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

        // 1. Get full session (messages + config)
        let messages =
            queries::chat::get_messages_by_file_id(&mut conn, self.workspace_id, self.chat_id)
                .await?;

        // 2. Hydrate session model
        let file = queries::files::get_file_by_id(&mut conn, self.chat_id).await?;

        let agent_config = if let Some(_version_id) = file.latest_version_id {
            let version = queries::files::get_latest_version(&mut conn, self.chat_id).await?;
            serde_json::from_value(version.app_data).unwrap_or_else(|_| {
                crate::models::chat::AgentConfig {
                    agent_id: None,
                    model: "gpt-4o".to_string(),
                    temperature: 0.7,
                    persona_override: None,
                }
            })
        } else {
            crate::models::chat::AgentConfig {
                agent_id: None,
                model: "gpt-4o".to_string(),
                temperature: 0.7,
                persona_override: None,
            }
        };

        let session = crate::models::chat::ChatSession {
            file_id: self.chat_id,
            agent_config,
            messages: messages.clone(),
        };

        // 3. Create Rig Agent
        let agent = self
            .rig_service
            .create_agent(self.pool.clone(), self.workspace_id, user_id, &session)
            .await?;

        // 4. Build prompt (the last message is the one we just saved in the handler)
        let last_message = messages
            .last()
            .ok_or_else(|| crate::error::Error::Internal("No messages found".into()))?;

        // 5. Convert history for Rig
        let history = self
            .rig_service
            .convert_history(&messages[..messages.len() - 1]);

        // 6. Stream from Rig
        let mut stream = agent
            .stream_chat(&last_message.content, history)
            .await;

        let mut full_response = String::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(rig::agent::MultiTurnStreamItem::StreamAssistantItem(content)) => {
                    match content {
                        rig::streaming::StreamedAssistantContent::Text(text) => {
                            full_response.push_str(&text.text);
                            let _ = self.event_tx.send(SseEvent::Chunk { text: text.text });
                        }
                        rig::streaming::StreamedAssistantContent::Reasoning(thought) => {
                            let _ = self.event_tx.send(SseEvent::Thought {
                                agent_id: None,
                                text: thought.reasoning.join("\n"),
                            });
                        }
                        rig::streaming::StreamedAssistantContent::ToolCall(tool_call) => {
                            let _ = self.event_tx.send(SseEvent::Call {
                                tool: tool_call.function.name,
                                path: None,
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
                            let _ = self.event_tx.send(SseEvent::Observation { output });
                        }
                    }
                }
                Err(e) => {
                    return Err(crate::error::Error::Llm(e.to_string()));
                }
                _ => {}
            }
        }

        // 7. Save Assistant Response
        let mut final_conn = self.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;
        queries::chat::insert_chat_message(
            &mut final_conn,
            NewChatMessage {
                file_id: self.chat_id,
                workspace_id: self.workspace_id,
                role: ChatMessageRole::Assistant,
                content: full_response,
                metadata: serde_json::Value::Object(serde_json::Map::new()),
            },
        )
        .await?;

        let _ = self.event_tx.send(SseEvent::Done {
            message: "Turn complete".to_string(),
        });

        Ok(())
    }
}
