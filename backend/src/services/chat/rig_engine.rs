use crate::models::chat::{ChatMessage, ChatMessageRole, ChatSession};
use crate::services::chat::rig_tools::{
    RigEditTool, RigGrepTool, RigLsTool, RigMkdirTool, RigMvTool, RigReadTool,
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
        ai_config: &crate::config::AiConfig,
    ) -> Result<rig::agent::Agent<ResponsesCompletionModel>> {
        use rig::client::CompletionClient;

        // 1. Resolve the model
        let model_name = if session.agent_config.model.is_empty() {
            openai::GPT_5_MINI  // Default to gpt-5-mini
        } else {
            &session.agent_config.model
        };

        // 2. Build the Rig Agent with Tools
        let persona = session.agent_config.persona_override.clone()
            .unwrap_or_else(|| crate::agents::get_persona(None));

        // 3. Build agent with optional reasoning parameters
        let agent_builder = self.client.agent(model_name)
            .preamble(&persona)
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
            .default_max_depth(DEFAULT_MAX_TOOL_ITERATIONS);

        // Add reasoning parameters based on configuration
        // Only add when enabled - OpenAI doesn't accept "none" as a valid value
        let agent_builder = if ai_config.enable_reasoning_summaries {
            agent_builder.additional_params(serde_json::json!({
                "reasoning": {
                    "effort": ai_config.reasoning_effort,
                    "summary": "auto"
                }
            }))
        } else {
            agent_builder
        };

        // Add previous_response_id if available (for conversation continuity with GPT-5)
        let agent_builder = if let Some(ref response_id) = session.agent_config.previous_response_id {
            agent_builder.additional_params(serde_json::json!({
                "previous_response_id": response_id
            }))
        } else {
            agent_builder
        };

        let agent = agent_builder.build();

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
