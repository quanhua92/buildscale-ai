use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub file_id: Uuid,
    pub workspace_id: Uuid,
    pub role: ChatMessageRole,
    pub content: String,
    pub metadata: ChatMessageMetadata,
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
    pub metadata: ChatMessageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_id: Option<Uuid>, // Points to an Agent File
    pub model: String,
    pub temperature: f32,
    pub persona_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub file_id: Uuid,
    pub agent_config: AgentConfig,
    pub messages: Vec<ChatMessage>,
}
