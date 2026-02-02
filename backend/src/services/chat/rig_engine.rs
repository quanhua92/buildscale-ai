use crate::models::chat::{ChatMessage, ChatMessageRole, ChatSession};
use crate::services::chat::rig_tools::{
    RigEditTool, RigGrepTool, RigLsTool, RigMkdirTool, RigMvTool, RigReadTool,
    RigRmTool, RigTouchTool, RigWriteTool,
    RigAskUserTool, RigExitPlanModeTool,
};
use crate::services::storage::FileStorageService;
use crate::providers::{AiProvider, Agent, ModelIdentifier, OpenAiProvider, OpenRouterProvider};
use crate::config::AiConfig;
use crate::DbPool;
use crate::error::{Error, Result};
use rig::client::CompletionClient;
use rig::completion::Message;
use rig::providers::openai::responses_api::ResponsesCompletionModel;
use std::sync::Arc;
use std::str::FromStr;
use uuid::Uuid;

/// Maximum number of tool-calling iterations allowed per user message.
/// This prevents infinite loops while allowing complex multi-step workflows.
const DEFAULT_MAX_TOOL_ITERATIONS: usize = 100;

/// Multi-provider AI service supporting OpenAI and OpenRouter
#[derive(Debug)]
pub struct RigService {
    openai: Option<Arc<OpenAiProvider>>,
    openrouter: Option<Arc<OpenRouterProvider>>,
    default_provider: AiProvider,
}

impl RigService {
    /// Create RigService from AiConfig (recommended method)
    pub fn from_config(ai_config: &AiConfig) -> Result<Self> {
        let default_provider = AiProvider::from_str(&ai_config.providers.default_provider)
            .map_err(|e| crate::error::Error::Internal(format!("Invalid default provider: {}", e)))?;

        // Initialize OpenAI provider if configured
        let openai = if let Some(openai_config) = &ai_config.providers.openai {
            let mut provider = OpenAiProvider::new(&openai_config.api_key, openai_config.base_url.as_deref());
            provider = provider.with_reasoning(
                openai_config.enable_reasoning_summaries,
                openai_config.reasoning_effort.clone()
            );
            Some(Arc::new(provider))
        } else {
            None
        };

        // Initialize OpenRouter provider if configured
        let openrouter = if let Some(openrouter_config) = &ai_config.providers.openrouter {
            let provider = OpenRouterProvider::new(&openrouter_config.api_key, openrouter_config.base_url.as_deref());
            Some(Arc::new(provider))
        } else {
            None
        };

        // Validate at least one provider is configured
        if openai.is_none() && openrouter.is_none() {
            return Err(crate::error::Error::Internal(
                "No AI providers configured".to_string()
            ));
        }

        // Validate default provider exists
        match default_provider {
            AiProvider::OpenAi if openai.is_none() => {
                return Err(crate::error::Error::Internal(
                    "Default provider is OpenAI, but OpenAI is not configured".to_string()
                ));
            }
            AiProvider::OpenRouter if openrouter.is_none() => {
                return Err(crate::error::Error::Internal(
                    "Default provider is OpenRouter, but OpenRouter is not configured".to_string()
                ));
            }
            _ => {}
        }

        Ok(RigService {
            openai,
            openrouter,
            default_provider,
        })
    }

    /// Create from legacy API key string (for backward compatibility)
    #[deprecated(note = "Use RigService::from_config() instead")]
    pub fn new(api_key: &str) -> Self {
        use secrecy::SecretString;

        // Create a simple provider config with just OpenAI
        let openai = Some(Arc::new(OpenAiProvider::new(
            &SecretString::new(api_key.to_string().into()),
            None,
        )));

        RigService {
            openai,
            openrouter: None,
            default_provider: AiProvider::OpenAi,
        }
    }

    /// Create from environment variables (for backward compatibility)
    #[deprecated(note = "Use RigService::from_config() instead")]
    pub fn from_env() -> Self {
        use rig::client::ProviderClient;
        let _client = rig::providers::openai::Client::from_env();

        // Since we can't extract the API key from the client, we create a dummy one
        // The client itself is already initialized from env
        // This is a compatibility shim - prefer from_config()
        use secrecy::SecretString;
        let openai = Some(Arc::new(OpenAiProvider::new(
            &SecretString::new("from_env".to_string().into()),
            None,
        )));

        RigService {
            openai,
            openrouter: None,
            default_provider: AiProvider::OpenAi,
        }
    }

    /// Creates a dummy RigService for testing purposes.
    pub fn dummy() -> Self {
        use secrecy::SecretString;
        let openai = Some(Arc::new(OpenAiProvider::new(
            &SecretString::new("sk-dummy".to_string().into()),
            None,
        )));

        RigService {
            openai,
            openrouter: None,
            default_provider: AiProvider::OpenAi,
        }
    }

    /// Get the default provider
    pub fn default_provider(&self) -> AiProvider {
        self.default_provider
    }

    /// Check if a provider is configured
    pub fn is_provider_configured(&self, provider: AiProvider) -> bool {
        match provider {
            AiProvider::OpenAi => self.openai.is_some(),
            AiProvider::OpenRouter => self.openrouter.is_some(),
        }
    }

    /// Get all configured providers
    pub fn configured_providers(&self) -> Vec<AiProvider> {
        let mut providers = Vec::new();
        if self.openai.is_some() {
            providers.push(AiProvider::OpenAi);
        }
        if self.openrouter.is_some() {
            providers.push(AiProvider::OpenRouter);
        }
        providers
    }

    /// Creates a Rig agent configured for the given chat session.
    /// Returns our unified Agent enum that wraps both OpenAI and OpenRouter agents.
    pub async fn create_agent(
        &self,
        pool: DbPool,
        storage: Arc<FileStorageService>,
        workspace_id: Uuid,
        chat_id: Uuid,
        user_id: Uuid,
        session: &ChatSession,
        ai_config: &AiConfig,
    ) -> Result<Agent> {
        // 1. Parse model identifier (supports both "provider:model" and legacy "model" formats)
        let model_id = ModelIdentifier::parse(
            &session.agent_config.model,
            self.default_provider
        ).map_err(|e| Error::Internal(format!("Invalid model format: {}", e)))?;

        // 2. Resolve model name (use default if empty)
        let model_name = if model_id.model.is_empty() {
            // Use provider-specific default
            match model_id.provider {
                AiProvider::OpenAi => "gpt-5-mini",
                AiProvider::OpenRouter => "anthropic/claude-3.5-sonnet",
            }
        } else {
            model_id.model.as_str()
        };

        // 3. Build persona (same for both providers)
        let persona = if let Some(ref override_persona) = session.agent_config.persona_override {
            override_persona.clone()
        } else {
            // Auto-select persona based on chat mode
            let mode = session.agent_config.mode.as_str();

            // For build mode, read plan content and inject into builder persona
            if mode == "build" {
                if let Some(ref plan_file_path) = session.agent_config.plan_file {
                    // Read plan file content
                    let mut conn = pool.acquire().await.map_err(|e| Error::Internal(format!("Database error: {}", e)))?;

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

        // 4. Create ToolConfig (same for both providers)
        let tool_config = crate::tools::ToolConfig {
            plan_mode: session.agent_config.mode == "plan",
            active_plan_path: session.agent_config.plan_file.clone(),
        };

        // 5. Build agent based on provider type
        match model_id.provider {
            AiProvider::OpenAi => {
                // Validate OpenAI provider is configured
                let openai_provider = self.openai.as_ref()
                    .ok_or_else(|| Error::Internal("OpenAI provider not configured".to_string()))?;

                // Build agent with OpenAI client
                let agent_builder = openai_provider.client().agent(model_name)
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
                let reasoning_enabled = openai_provider.is_reasoning_enabled();

                if reasoning_enabled {
                    if let Some(obj) = params.as_object_mut() {
                        obj.insert("reasoning".to_string(), serde_json::json!({
                            "effort": openai_provider.reasoning_effort(),
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
                Ok(crate::providers::Agent::OpenAI(agent))
            }
            AiProvider::OpenRouter => {
                // Validate OpenRouter provider is configured
                let openrouter_provider = self.openrouter.as_ref()
                    .ok_or_else(|| Error::Internal("OpenRouter provider not configured".to_string()))?;

                // Build agent with OpenRouter client
                let agent_builder = openrouter_provider.client().agent(model_name)
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

                tracing::info!(
                    "Built OpenRouter agent with model: {}",
                    model_name
                );

                let agent = agent_builder.build();
                Ok(crate::providers::Agent::OpenRouter(agent))
            }
        }
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
