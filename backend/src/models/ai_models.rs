//! AI models and workspace model access control

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// AI model from any provider
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AiModel {
    pub id: Uuid,
    pub provider: String,           // 'openai', 'openrouter', etc.
    pub model_name: String,         // e.g., 'gpt-4o', 'anthropic/claude-3.5-sonnet'
    pub display_name: String,       // e.g., 'GPT-4o'
    pub description: Option<String>,
    pub context_window: Option<i32>,
    pub is_enabled: bool,           // Global enable/disable flag
    pub is_free: bool,              // Whether model is available for free
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new AI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAiModel {
    pub provider: String,
    pub model_name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub context_window: Option<i32>,
    pub is_enabled: bool,
    pub is_free: bool,
}

/// Update an existing AI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAiModel {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub context_window: Option<i32>,
    pub is_enabled: Option<bool>,
    pub is_free: Option<bool>,
}

/// Workspace-model mapping with access control
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceAiModel {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub model_id: Uuid,
    pub status: String,             // 'active', 'disabled', 'restricted'
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new workspace-model mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkspaceAiModel {
    pub workspace_id: Uuid,
    pub model_id: Uuid,
    pub status: String,
}

/// Update workspace-model mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceAiModel {
    pub status: Option<String>,
}

/// Status values for workspace-model access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelAccessStatus {
    Active,
    Disabled,
    Restricted,
}

impl ModelAccessStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelAccessStatus::Active => "active",
            ModelAccessStatus::Disabled => "disabled",
            ModelAccessStatus::Restricted => "restricted",
        }
    }
}

impl AsRef<str> for ModelAccessStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ModelAccessStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ModelAccessStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(ModelAccessStatus::Active),
            "disabled" => Ok(ModelAccessStatus::Disabled),
            "restricted" => Ok(ModelAccessStatus::Restricted),
            _ => Err(format!("Invalid model access status: {}", s)),
        }
    }
}
