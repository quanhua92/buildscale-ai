use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

/// Agent type enum - represents different AI agent personas
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, sqlx::Type,
)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AgentType {
    /// Standard chat assistant agent
    Assistant,
    /// Planning-focused agent for task breakdown
    Planner,
    /// Builder agent for code generation and execution
    Builder,
}

/// Session status enum - tracks the current state of an agent session
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, sqlx::Type,
)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SessionStatus {
    /// Agent is idle, waiting for input
    Idle,
    /// Agent is actively processing
    Running,
    /// Agent session is paused
    Paused,
    /// Agent session completed successfully
    Completed,
    /// Agent session encountered an error
    Error,
}

/// Agent session entity - represents an active AI agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub agent_type: AgentType,
    pub status: SessionStatus,
    pub model: String,
    pub mode: String,
    pub current_task: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// New agent session entity - for creating a new session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAgentSession {
    pub workspace_id: Uuid,
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub agent_type: AgentType,
    pub model: String,
    pub mode: String,
}

/// Update agent session entity - for updating session fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentSession {
    pub status: Option<SessionStatus>,
    pub current_task: Option<Option<String>>,
    pub completed_at: Option<Option<DateTime<Utc>>>,
}

/// Agent session heartbeat update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeartbeat {
    pub last_heartbeat: DateTime<Utc>,
}

/// Public agent session info (for API responses - excludes internal fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionInfo {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub agent_type: AgentType,
    pub status: SessionStatus,
    pub model: String,
    pub mode: String,
    pub current_task: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<AgentSession> for AgentSessionInfo {
    fn from(session: AgentSession) -> Self {
        Self {
            id: session.id,
            workspace_id: session.workspace_id,
            chat_id: session.chat_id,
            user_id: session.user_id,
            agent_type: session.agent_type,
            status: session.status,
            model: session.model,
            mode: session.mode,
            current_task: session.current_task,
            created_at: session.created_at,
            updated_at: session.updated_at,
            last_heartbeat: session.last_heartbeat,
            completed_at: session.completed_at,
        }
    }
}

/// List sessions response - contains sessions and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionsListResponse {
    pub sessions: Vec<AgentSessionInfo>,
    pub total: usize,
}

/// Pause session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseSessionRequest {
    pub reason: Option<String>,
}

/// Resume session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeSessionRequest {
    /// Optional task to resume with
    pub task: Option<String>,
}

/// Session action response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionActionResponse {
    pub session: AgentSessionInfo,
    pub message: String,
}
