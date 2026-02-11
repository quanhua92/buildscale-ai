use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

/// Default AI model for new chat sessions
pub const DEFAULT_CHAT_MODEL: &str = "gpt-5-mini";

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, sqlx::Type,
)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ChatMessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ChatAttachment {
    File {
        file_id: Uuid,
        version_id: Option<Uuid>,
    },
    Url {
        url: String,
        title: Option<String>,
    },
    Agent {
        agent_id: Uuid,
        name: String,
    },
    Skill {
        skill_id: Uuid,
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatMessageMetadata {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<ChatAttachment>,
    pub tool_calls: Option<serde_json::Value>,
    pub usage: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>, // Model that generated this message (e.g., "gpt-5", "gpt-5-mini", "gpt-4o")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>, // OpenAI Responses API response_id for conversation continuity
    /// Question answer metadata (from ask_user tool responses)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_answer: Option<QuestionAnswerMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_arguments: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_success: Option<bool>,
}

/// Question answer metadata stored with user messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionAnswerMetadata {
    pub question_id: Uuid,
    pub answers: serde_json::Value, // Object mapping question names to answers
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub file_id: Uuid,
    pub workspace_id: Uuid,
    pub role: ChatMessageRole,
    pub content: String,
    pub metadata: sqlx::types::Json<ChatMessageMetadata>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewChatMessage {
    pub file_id: Uuid,
    pub workspace_id: Uuid,
    pub role: ChatMessageRole,
    pub content: String,
    pub metadata: sqlx::types::Json<ChatMessageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_id: Option<Uuid>, // Points to an Agent File
    pub model: String,
    pub temperature: f32,
    pub persona_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>, // OpenAI Responses API response_id for conversation continuity
    /// Chat mode: "plan" or "build" (default: "plan")
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Absolute path to associated .plan file (only in build mode)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_file: Option<String>,
}

fn default_mode() -> String {
    "plan".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub file_id: Uuid,
    pub agent_config: AgentConfig,
    pub messages: Vec<ChatMessage>,
}

// ============================================================================
// Context API Response Models
// ============================================================================

/// Response for GET /chat/{id}/context - detailed AI context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContextResponse {
    pub system_prompt: SystemPromptSection,
    pub history: HistorySection,
    pub tools: ToolsSection,
    pub attachments: AttachmentsSection,
    pub summary: ContextSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPromptSection {
    pub content: String,
    pub char_count: usize,
    pub token_count: usize,
    pub persona_type: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySection {
    pub messages: Vec<HistoryMessageInfo>,
    pub message_count: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryMessageInfo {
    pub role: String,
    pub content_preview: String,
    pub content_length: usize,
    pub token_count: usize,
    pub metadata: Option<HistoryMessageMetadata>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryMessageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsSection {
    pub tools: Vec<ToolDefinition>,
    pub tool_count: usize,
    pub estimated_schema_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentsSection {
    pub attachments: Vec<AttachmentInfo>,
    pub attachment_count: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub attachment_type: String,
    pub id: Uuid,
    pub content_preview: String,
    pub content_length: usize,
    pub token_count: usize,
    pub priority: i32,
    pub is_essential: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    pub total_tokens: usize,
    pub utilization_percent: f64,
    pub model: String,
    pub token_limit: usize,
    pub breakdown: TokenBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBreakdown {
    pub system_prompt_tokens: usize,
    pub history_tokens: usize,
    pub tools_tokens: usize,
    pub attachments_tokens: usize,
}
