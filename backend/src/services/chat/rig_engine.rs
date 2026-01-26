use crate::models::chat::{ChatMessage, ChatMessageRole, ChatSession};
use crate::services::chat::rig_tools::{
    RigEditManyTool, RigEditTool, RigGrepTool, RigLsTool, RigMkdirTool, RigMvTool, RigReadTool,
    RigRmTool, RigTouchTool, RigWriteTool,
};
use crate::DbPool;
use crate::error::Result;
use rig::providers::openai::{self, responses_api::ResponsesCompletionModel};
use rig::completion::Message;
use uuid::Uuid;

/// Maximum number of tool-calling iterations allowed per user message.
/// This prevents infinite loops while allowing complex multi-step workflows.
const DEFAULT_MAX_TOOL_ITERATIONS: usize = 100;

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

    /// Creates a dummy RigService for testing purposes.
    pub fn dummy() -> Self {
        Self {
            client: openai::Client::new("sk-dummy").expect("Failed to create OpenAI client"),
        }
    }

    /// Creates a Rig agent configured for the given chat session.
    pub async fn create_agent(
        &self,
        pool: DbPool,
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
            .preamble(session.agent_config.persona_override.as_deref().unwrap_or_else(|| crate::agents::get_persona(None)))
            .tool(RigLsTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigReadTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigWriteTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigRmTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigMvTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigTouchTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigEditTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigEditManyTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigGrepTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigMkdirTool {
                pool: pool.clone(),
                workspace_id,
                user_id,
            })
            .default_max_depth(DEFAULT_MAX_TOOL_ITERATIONS)
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
