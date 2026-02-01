use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub role_name: Option<String>,
    /// Optional per-workspace provider override
    /// If None, uses the global default provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_provider_override: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkspace {
    pub name: String,
    pub owner_id: Uuid,
    /// Optional per-workspace provider override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_provider_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspace {
    pub name: Option<String>,
    pub owner_id: Option<Uuid>,
    /// Optional per-workspace provider override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_provider_override: Option<Option<String>>,
}
