use crate::models::chat::{ChatMessage, ChatMessageRole, ChatSession};
use crate::services::chat::rig_tools::{
    RigEditTool, RigGrepTool, RigLsTool, RigMkdirTool, RigMvTool, RigReadTool,
    RigRmTool, RigTouchTool, RigWriteTool,
};
use crate::services::storage::FileStorageService;
use crate::DbPool;
use crate::error::Result;
use rig::providers::openai::{self, responses_api::ResponsesCompletionModel};
use rig::completion::Message;
use std::sync::Arc;
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
        storage: Arc<FileStorageService>,
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
                storage: storage.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigReadTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigWriteTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigRmTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigMvTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigTouchTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigEditTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigGrepTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                user_id,
            })
            .tool(RigMkdirTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                user_id,
            })
            .default_max_depth(DEFAULT_MAX_TOOL_ITERATIONS);

        // Build additional parameters for OpenAI Responses API
        // CRITICAL: Set store: false to use stateless mode
        // This prevents OpenAI from requiring reasoning items to be maintained across requests
        // Without this, Rig loses reasoning items when managing chat_history, causing 400 errors
        let mut params = serde_json::json!({
            "store": false,
            "reasoning": {
                "effort": ai_config.reasoning_effort
            }
        });

        // Enable reasoning summaries based on configuration
        if ai_config.enable_reasoning_summaries {
            if let Some(obj) = params.get_mut("reasoning") {
                if let Some(reasoning_obj) = obj.as_object_mut() {
                    reasoning_obj.insert("summary".to_string(), serde_json::json!("auto"));
                }
            }
        }

        let agent_builder = agent_builder.additional_params(params);

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
