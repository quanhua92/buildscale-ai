use crate::models::chat::{ChatMessage, ChatMessageRole, ChatSession};
use crate::services::chat::rig_tools::{
    RigEditTool, RigGrepTool, RigLsTool, RigMkdirTool, RigMvTool, RigReadTool,
    RigRmTool, RigTouchTool, RigWriteTool,
    RigAskUserTool, RigExitPlanModeTool,
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
        chat_id: Uuid,
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
        // Select persona based on mode (plan vs build)
        let persona = if let Some(ref override_persona) = session.agent_config.persona_override {
            override_persona.clone()
        } else {
            // Auto-select persona based on chat mode
            let mode = session.agent_config.mode.as_str();

            // For build mode, read plan content and inject into builder persona
            if mode == "build" {
                if let Some(ref plan_file_path) = session.agent_config.plan_file {
                    // Read plan file content
                    let mut conn = pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

                    // Get plan file
                    if let Ok(Some(plan_file)) = crate::queries::files::get_file_by_path(
                        &mut conn, workspace_id, plan_file_path
                    ).await {
                        if let Ok(plan_with_content) = crate::services::files::get_file_with_content(
                            &mut conn, &storage, plan_file.id
                        ).await {
                            let plan_content = plan_with_content.content.to_string();
                            // Builder persona with plan content
                            crate::agents::get_persona(Some("builder"), None, Some(&plan_content))
                        } else {
                            // Failed to read plan, use builder without plan
                            crate::agents::get_persona(Some("builder"), None, Some("# Error: Could not read plan file"))
                        }
                    } else {
                        // Plan file not found, use builder without plan
                        crate::agents::get_persona(Some("builder"), None, Some("# Error: Plan file not found"))
                    }
                } else {
                    // No plan file specified, use builder without plan
                    crate::agents::get_persona(Some("builder"), None, Some("# Error: No plan file specified in build mode"))
                }
            } else {
                // Plan mode or default
                crate::agents::get_persona(None, Some(mode), None)
            }
        };

        // TODO: Phase 3 - Extract ToolConfig from chat metadata
        // For now, derive ToolConfig from agent_config.mode and agent_config.plan_file
        let tool_config = crate::tools::ToolConfig {
            plan_mode: session.agent_config.mode == "plan",
            active_plan_path: session.agent_config.plan_file.clone(),
        };

        // 3. Build agent with optional reasoning parameters
        let agent_builder = self.client.agent(model_name)
            .preamble(&persona)
            .tool(RigLsTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigReadTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigWriteTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigRmTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigMvTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigTouchTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigEditTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigGrepTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigMkdirTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigAskUserTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .tool(RigExitPlanModeTool {
                pool: pool.clone(),
                storage: storage.clone(),
                workspace_id,
                chat_id,
                user_id,
                tool_config: tool_config.clone(),
            })
            .default_max_depth(DEFAULT_MAX_TOOL_ITERATIONS);

        // Build additional parameters for OpenAI Responses API
        // CRITICAL: Set store: false to use stateless mode
        // This prevents OpenAI from requiring reasoning items to be maintained across requests
        // Without this, Rig loses reasoning items when managing chat_history, causing 400 errors
        let mut params = serde_json::json!({
            "store": false
        });

        // Enable reasoning with effort and summaries based on configuration
        // Check if OpenAI provider has reasoning enabled
        let reasoning_enabled = ai_config.providers.openai
            .as_ref()
            .map(|config| config.enable_reasoning_summaries)
            .unwrap_or(false);

        let reasoning_effort = ai_config.providers.openai
            .as_ref()
            .map(|config| config.reasoning_effort.clone())
            .unwrap_or_else(|| "low".to_string());

        if reasoning_enabled {
            if let Some(obj) = params.as_object_mut() {
                obj.insert("reasoning".to_string(), serde_json::json!({
                    "effort": reasoning_effort,
                    "summary": "auto"
                }));
            }
        }

        tracing::info!(
            "Agent additional_params: {}",
            serde_json::to_string(&params).unwrap_or_else(|_| "INVALID".to_string())
        );

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
