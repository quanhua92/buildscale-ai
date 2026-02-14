use crate::models::chat::{ChatMessage, ChatMessageRole, ChatSession};
use crate::services::chat::rig_tools::{
    RigEditTool, RigGrepTool, RigGlobTool, RigFileInfoTool, RigLsTool, RigMkdirTool, RigMvTool, RigReadTool,
    RigRmTool, RigTouchTool, RigWriteTool, RigReadMultipleFilesTool, RigFindTool, RigCatTool,
    RigAskUserTool, RigExitPlanModeTool,
    RigPlanWriteTool, RigPlanReadTool, RigPlanEditTool, RigPlanListTool,
    RigMemorySetTool, RigMemoryGetTool, RigMemorySearchTool, RigMemoryDeleteTool,
};
use crate::services::chat::context::{
    build_sorted_context_items, get_indices_to_truncate, render_attachment_for_ai,
    truncate_tool_output, AttachmentManager, ContextItem,
};
use crate::services::storage::FileStorageService;
use crate::providers::{AiProvider, Agent, ModelIdentifier, OpenAiProvider, OpenRouterProvider};
use crate::config::AiConfig;
use crate::DbPool;
use crate::error::{Error, Result};
use rig::client::CompletionClient;
use rig::completion::Message;
use std::sync::Arc;
use std::str::FromStr;
use uuid::Uuid;

/// Maximum number of tool-calling iterations allowed per user message.
/// This prevents infinite loops while allowing complex multi-step workflows.
const DEFAULT_MAX_TOOL_ITERATIONS: usize = 100;

/// Add all Rig tools to an agent builder
fn add_tools_to_agent<M>(
    builder: rig::agent::AgentBuilder<M>,
    pool: &DbPool,
    storage: &Arc<FileStorageService>,
    workspace_id: Uuid,
    chat_id: Uuid,
    user_id: Uuid,
    tool_config: &crate::tools::ToolConfig,
) -> rig::agent::AgentBuilderSimple<M>
where
    M: rig::completion::CompletionModel + 'static,
{
    builder
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
        .tool(RigEditTool {
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
        .tool(RigMkdirTool {
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
        .tool(RigGlobTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigFileInfoTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigReadMultipleFilesTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigFindTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigCatTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigPlanWriteTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigPlanReadTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigPlanEditTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigPlanListTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigMemorySetTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigMemoryGetTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigMemorySearchTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .tool(RigMemoryDeleteTool {
            pool: pool.clone(),
            storage: storage.clone(),
            workspace_id,
            chat_id,
            user_id,
            tool_config: tool_config.clone(),
        })
        .default_max_depth(DEFAULT_MAX_TOOL_ITERATIONS)
}

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
            AiProvider::OpenAi | AiProvider::OpenRouter => {
                // Valid configuration, continue
            }
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
        _ai_config: &AiConfig,
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
                    .preamble(&persona);
                let agent_builder = add_tools_to_agent(
                    agent_builder,
                    &pool,
                    &storage,
                    workspace_id,
                    chat_id,
                    user_id,
                    &tool_config,
                );

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
                    .preamble(&persona);
                let agent_builder = add_tools_to_agent(
                    agent_builder,
                    &pool,
                    &storage,
                    workspace_id,
                    chat_id,
                    user_id,
                    &tool_config,
                );

                tracing::info!(
                    "Built OpenRouter agent with model: {}",
                    model_name
                );

                let agent = agent_builder.build();
                Ok(crate::providers::Agent::OpenRouter(agent))
            }
        }
    }

    /// Reconstruct a ToolCall message from metadata.
    ///
    /// This helper extracts tool call information from metadata
    /// and reconstructs it as a Rig ToolCall message. Used by convert_history
    /// to avoid duplicating this logic across multiple match arms.
    fn reconstruct_tool_call_from_metadata(
        &self,
        metadata: &crate::models::chat::ChatMessageMetadata,
    ) -> Option<Message> {
        use rig::completion::AssistantContent;
        use rig::message::{ToolCall, ToolFunction};

        let tool_id = metadata.reasoning_id.clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        if let (Some(tool_name), Some(tool_args)) =
            (&metadata.tool_name, &metadata.tool_arguments)
        {
            Some(Message::Assistant {
                id: None,
                content: rig::OneOrMany::one(
                    AssistantContent::ToolCall(ToolCall {
                        id: tool_id,
                        call_id: None,
                        function: ToolFunction {
                            name: tool_name.clone(),
                            arguments: tool_args.clone(),
                        },
                        signature: None,
                        additional_params: None,
                    })
                )
            })
        } else {
            None
        }
    }

    /// Reconstruct a ToolResult message from metadata.
    ///
    /// This helper extracts tool result information from metadata
    /// and reconstructs it as a Rig ToolResult message. Used by convert_history
    /// to avoid duplicating this logic across multiple match arms.
    fn reconstruct_tool_result_from_metadata(
        &self,
        metadata: &crate::models::chat::ChatMessageMetadata,
        include_note: bool,
    ) -> Option<Message> {
        use rig::message::{UserContent, ToolResult, ToolResultContent};
        use rig::message::Text;

        let tool_id = metadata.reasoning_id.clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let tool_output = if let Some(ref output) = metadata.tool_output {
            if include_note {
                format!("{}\n[Note: This is a summarized tool result, not the full output]",
                    output)
            } else {
                output.clone()
            }
        } else if include_note {
            "[Note: Tool result was not available]".to_string()
        } else {
            "[No output]".to_string()
        };

        Some(Message::User {
            content: rig::OneOrMany::one(
                UserContent::ToolResult(ToolResult {
                    id: tool_id,
                    call_id: None,
                    content: rig::OneOrMany::one(ToolResultContent::Text(Text { text: tool_output })),
                })
            )
        })
    }

    /// Converts BuildScale chat history to Rig messages.
    /// This includes tool calls and tool results which are critical for multi-turn conversations.
    ///
    /// IMPORTANT: Tool results are summarized in the database (not full outputs).
    /// We include these summaries with a note that they are truncated.
    pub fn convert_history(&self, messages: &[ChatMessage]) -> Vec<Message> {
        self.convert_history_with_attachments(messages, None)
    }

    /// Converts BuildScale chat history to Rig messages with cache-optimized attachment interleaving.
    ///
    /// This method interleaves attachments chronologically with messages to create a stable
    /// cacheable prefix. Older content (both messages and attachments) becomes part of the
    /// stable prefix, while newer content varies only at the end.
    ///
    /// # Cache Optimization
    ///
    /// Before (current - bad for caching):
    /// ```text
    /// [System Prompt] → [History] → [Last Message] → [Attachments]
    ///                                      ↑                    ↑
    ///                                   varies              varies
    /// ```
    ///
    /// After (cache-optimized):
    /// ```text
    /// [System Prompt] → [Interleaved History + Attachments] → [Last Message]
    ///                              ↑                              ↑
    ///                         cacheable                      varies only
    /// ```
    ///
    /// # Tool Result Optimization
    ///
    /// Tool results can be very large. To reduce context size, we truncate outputs for
    /// tool results that are older than `KEEP_RECENT_TOOL_RESULTS` turns. The AI can
    /// re-run tools if it needs fresh data.
    ///
    /// # Arguments
    /// * `messages` - Chat message history (excluding current message)
    /// * `attachments` - Optional attachment manager with file contents
    ///
    /// # Returns
    /// Vector of Rig messages ready for the AI, with attachments interleaved chronologically
    pub fn convert_history_with_attachments(
        &self,
        messages: &[ChatMessage],
        attachments: Option<&AttachmentManager>,
    ) -> Vec<Message> {
        // Use centralized context building from context.rs (single source of truth)
        let items = build_sorted_context_items(messages, attachments, Some(render_attachment_for_ai));

        // Use centralized truncation logic
        let indices_to_truncate = get_indices_to_truncate(&items);

        // Convert to Rig Messages
        let mut result = Vec::new();
        for (idx, item) in items.into_iter().enumerate() {
            match item {
                ContextItem::Message { role, content, mut metadata, .. } => {
                    // Truncate tool output for old tool results using shared function
                    if indices_to_truncate.contains(&idx) {
                        if let Some(ref tool_output) = metadata.tool_output {
                            metadata.tool_output = Some(truncate_tool_output(tool_output));
                        }
                    }
                    // Reuse existing message conversion logic
                    if let Some(msg) = Self::convert_single_message(role, content, metadata, self) {
                        result.push(msg);
                    }
                }
                ContextItem::Attachment { rendered, .. } => {
                    // Attachments become user messages (context) in the history
                    // Using user message as Rig doesn't have system messages in history
                    result.push(Message::user(format!("[Context]\n{}", rendered)));
                }
            }
        }

        result
    }

    /// Convert a single message to Rig Message format
    fn convert_single_message(
        role: ChatMessageRole,
        content: String,
        metadata: crate::models::chat::ChatMessageMetadata,
        service: &RigService,
    ) -> Option<Message> {
        match role {
            ChatMessageRole::User => {
                // Check if this is a tool result (message_type: "tool_result")
                if let Some(ref message_type) = metadata.message_type {
                    match message_type.as_str() {
                        "tool_result" => {
                            // Reconstruct ToolResult directly from metadata (no temp allocation)
                            service.reconstruct_tool_result_from_metadata(&metadata, true)
                        }
                        _ => {
                            // Regular user text message
                            Some(Message::user(content))
                        }
                    }
                } else {
                    // Regular user text message (no metadata)
                    Some(Message::user(content))
                }
            }
            ChatMessageRole::Assistant => {
                // Check if this is a tool call (message_type: "tool_call")
                if let Some(ref message_type) = metadata.message_type {
                    match message_type.as_str() {
                        "tool_call" => {
                            // Reconstruct ToolCall directly from metadata (no temp allocation)
                            service.reconstruct_tool_call_from_metadata(&metadata)
                                .or_else(|| Some(Message::assistant(content)))
                        }
                        _ => {
                            // Regular assistant text message
                            Some(Message::assistant(content))
                        }
                    }
                } else {
                    // Regular assistant text message (no metadata)
                    Some(Message::assistant(content))
                }
            }
            ChatMessageRole::System => {
                // System messages typically not sent in chat history
                None
            }
            ChatMessageRole::Tool => {
                // Convert Tool role messages to Rig format
                if let Some(ref message_type) = metadata.message_type {
                    match message_type.as_str() {
                        "tool_call" => {
                            service.reconstruct_tool_call_from_metadata(&metadata)
                        }
                        "tool_result" => {
                            service.reconstruct_tool_result_from_metadata(&metadata, false)
                        }
                        unknown_type => {
                            tracing::error!(
                                message_type = %unknown_type,
                                "Unknown message_type '{}' in Tool role, filtered from AI context. \
                                 This may cause DATA LOSS. Add explicit handling if this is a valid message type.",
                                unknown_type
                            );
                            None
                        }
                    }
                } else {
                    None  // Tool role without message_type metadata
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::{ChatMessage, ChatMessageMetadata, ChatMessageRole};
    use chrono::Utc;
    use rig::message::{UserContent, AssistantContent};
    use rig::completion::message::ToolResultContent;

    #[test]
    fn test_convert_history_with_tool_calls_and_results() {
        let service = RigService::dummy();

        // Create a user message
        let user_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::User,
            content: "Hello".to_string(),
            metadata: ChatMessageMetadata::default().into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        // Create a tool call message (assistant role with tool_call metadata)
        let tool_call_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::Assistant,
            content: "AI called tool: read".to_string(),
            metadata: ChatMessageMetadata {
                message_type: Some("tool_call".to_string()),
                reasoning_id: Some("test-tool-id-123".to_string()),
                tool_name: Some("read".to_string()),
                tool_arguments: Some(serde_json::json!({"path": "/tmp/test.txt"})),
                ..Default::default()
            }.into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        // Create a tool result message (user role with tool_result metadata)
        let tool_result_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::User,
            content: "Tool read: succeeded".to_string(),
            metadata: ChatMessageMetadata {
                message_type: Some("tool_result".to_string()),
                reasoning_id: Some("test-tool-id-123".to_string()),
                tool_output: Some("Line 1\nLine 2\nLine 3".to_string()),
                tool_success: Some(true),
                ..Default::default()
            }.into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        // Create an assistant text message
        let assistant_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::Assistant,
            content: "Here's what I found".to_string(),
            metadata: ChatMessageMetadata::default().into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        let messages = vec![
            user_msg,
            tool_call_msg,
            tool_result_msg,
            assistant_msg,
        ];

        let converted = service.convert_history(&messages);

        // We should have 4 messages in the history
        assert_eq!(converted.len(), 4, "Should convert all messages including tool calls and results");

        // First message: regular user text
        match &converted[0] {
            rig::completion::Message::User { content } => {
                match content.iter().next() {
                    Some(UserContent::Text(text)) => {
                        assert_eq!(text.text, "Hello");
                    }
                    _ => panic!("Expected text content for user message"),
                }
            }
            _ => panic!("Expected User message"),
        }

        // Second message: assistant tool call
        match &converted[1] {
            rig::completion::Message::Assistant { content, .. } => {
                match content.iter().next() {
                    Some(AssistantContent::ToolCall(tool_call)) => {
                        assert_eq!(tool_call.function.name, "read");
                        assert_eq!(tool_call.id, "test-tool-id-123");
                    }
                    _ => panic!("Expected ToolCall content for assistant message"),
                }
            }
            _ => panic!("Expected Assistant message"),
        }

        // Third message: user tool result
        match &converted[2] {
            rig::completion::Message::User { content } => {
                match content.iter().next() {
                    Some(UserContent::ToolResult(result)) => {
                        assert_eq!(result.id, "test-tool-id-123");
                        match result.content.iter().next() {
                            Some(ToolResultContent::Text(text)) => {
                                assert!(text.text.contains("Line 1"));
                                assert!(text.text.contains("[Note: This is a summarized tool result"));
                            }
                            _ => panic!("Expected Text content in tool result"),
                        }
                    }
                    _ => panic!("Expected ToolResult content for user message"),
                }
            }
            _ => panic!("Expected User message"),
        }

        // Fourth message: regular assistant text
        match &converted[3] {
            rig::completion::Message::Assistant { content, .. } => {
                match content.iter().next() {
                    Some(AssistantContent::Text(text)) => {
                        assert_eq!(text.text, "Here's what I found");
                    }
                    _ => panic!("Expected text content for assistant message"),
                }
            }
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_convert_history_converts_tool_role_messages() {
        let service = RigService::dummy();

        // Create a Tool role message with tool_call metadata
        let tool_call_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::Tool,
            content: "Tool call: ls".to_string(),
            metadata: ChatMessageMetadata {
                message_type: Some("tool_call".to_string()),
                reasoning_id: Some("test-tool-id-456".to_string()),
                tool_name: Some("ls".to_string()),
                tool_arguments: Some(serde_json::json!({"path": "/tmp"})),
                ..Default::default()
            }.into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        // Create a Tool role message with tool_result metadata
        let tool_result_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::Tool,
            content: "Tool result: success".to_string(),
            metadata: ChatMessageMetadata {
                message_type: Some("tool_result".to_string()),
                reasoning_id: Some("test-tool-id-456".to_string()),
                tool_output: Some("file1.txt\nfile2.txt".to_string()),
                tool_success: Some(true),
                ..Default::default()
            }.into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        // Create a system message (should be filtered out)
        let system_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::System,
            content: "System prompt".to_string(),
            metadata: ChatMessageMetadata::default().into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        let messages = vec![tool_call_msg, tool_result_msg, system_msg];
        let converted = service.convert_history(&messages);

        // Should have 2 messages: tool_call and tool_result (system filtered out)
        assert_eq!(converted.len(), 2, "Should convert Tool role messages with metadata and filter System role");

        // First message: Tool call (as Assistant)
        match &converted[0] {
            rig::completion::Message::Assistant { content, .. } => {
                match content.iter().next() {
                    Some(AssistantContent::ToolCall(tool_call)) => {
                        assert_eq!(tool_call.function.name, "ls");
                        assert_eq!(tool_call.id, "test-tool-id-456");
                    }
                    _ => panic!("Expected ToolCall content"),
                }
            }
            _ => panic!("Expected Assistant message for tool call"),
        }

        // Second message: Tool result (as User)
        match &converted[1] {
            rig::completion::Message::User { content } => {
                match content.iter().next() {
                    Some(UserContent::ToolResult(result)) => {
                        assert_eq!(result.id, "test-tool-id-456");
                        match result.content.iter().next() {
                            Some(ToolResultContent::Text(text)) => {
                                assert!(text.text.contains("file1.txt"));
                            }
                            _ => panic!("Expected Text content in tool result"),
                        }
                    }
                    _ => panic!("Expected ToolResult content"),
                }
            }
            _ => panic!("Expected User message for tool result"),
        }
    }

    #[test]
    fn test_convert_history_filters_reasoning_messages() {
        let service = RigService::dummy();

        // Create a reasoning message (Assistant role with reasoning_complete metadata)
        let reasoning_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::Assistant,
            content: "I need to think about this...".to_string(),
            metadata: ChatMessageMetadata {
                message_type: Some("reasoning_complete".to_string()),
                ..Default::default()
            }.into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        // Create a reasoning message (User role with reasoning_complete metadata)
        let reasoning_user_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::User,
            content: "User reasoning...".to_string(),
            metadata: ChatMessageMetadata {
                message_type: Some("reasoning_complete".to_string()),
                ..Default::default()
            }.into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        // Create a regular user message
        let user_msg = ChatMessage {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            role: ChatMessageRole::User,
            content: "Hello".to_string(),
            metadata: ChatMessageMetadata::default().into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        let messages = vec![reasoning_msg, reasoning_user_msg, user_msg];
        let converted = service.convert_history(&messages);

        // Should have only 1 message (the regular user message), reasoning messages filtered
        assert_eq!(converted.len(), 1, "Reasoning messages should be filtered from AI context");

        // First message: regular user text
        match &converted[0] {
            rig::completion::Message::User { content } => {
                match content.iter().next() {
                    Some(UserContent::Text(text)) => {
                        assert_eq!(text.text, "Hello");
                    }
                    _ => panic!("Expected text content for user message"),
                }
            }
            _ => panic!("Expected User message"),
        }
    }
}
