use crate::models::chat::{ChatMessage, ChatMessageRole, ChatSession};
use crate::services::chat::rig_tools::{RigLsTool, RigReadTool, RigRmTool, RigWriteTool};
use crate::DbConn;
use crate::error::Result;
use rig::providers::openai::{self, responses_api::ResponsesCompletionModel};
use rig::completion::Message;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct RigService {
    client: openai::Client,
}

impl RigService {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: openai::Client::new(api_key).expect("Failed to create OpenAI client"),
        }
    }

    pub fn from_env() -> Self {
        use rig::client::ProviderClient;
        Self {
            client: openai::Client::from_env(),
        }
    }

    /// Creates a Rig agent configured for the given chat session.
    pub async fn create_agent(
        &self,
        conn: Arc<Mutex<DbConn>>,
        workspace_id: Uuid,
        user_id: Uuid,
        session: &ChatSession,
    ) -> Result<rig::agent::Agent<ResponsesCompletionModel>> {
        use rig::client::CompletionClient;

        // 1. Resolve the model
        let model_name = if session.agent_config.model.is_empty() {
            openai::GPT_4O
        } else {
            &session.agent_config.model
        };

        // 2. Build the Rig Agent with Tools
        let agent = self.client.agent(model_name)
            .preamble("You are BuildScale AI, a professional software engineering assistant.")
            .tool(RigLsTool {
                conn: conn.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigReadTool {
                conn: conn.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigWriteTool {
                conn: conn.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigRmTool {
                conn: conn.clone(),
                workspace_id,
                user_id,
            })
            .build();

        Ok(agent)
    }

    /// Converts BuildScale chat history to Rig messages.
    pub fn convert_history(&self, messages: &[ChatMessage]) -> Vec<Message> {
        messages
            .iter()
            .filter_map(|msg| match msg.role {
                ChatMessageRole::User => Some(Message::user(msg.content.clone())),
                ChatMessageRole::Assistant => Some(Message::assistant(msg.content.clone())),
                _ => None, // System and Tool roles handled differently in Rig
            })
            .collect()
    }
}
