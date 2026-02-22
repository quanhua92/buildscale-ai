use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SseEvent {
    SessionInit {
        chat_id: Uuid,
        plan_id: Option<Uuid>,
    },
    Thought {
        agent_id: Option<Uuid>,
        text: String,
    },
    Call {
        tool: String,
        path: Option<String>,
        args: serde_json::Value,
    },
    Observation {
        output: String,
        success: bool,
    },
    FileUpdated {
        path: String,
        version: i32,
    },
    Chunk {
        text: String,
    },
    Error {
        message: String,
    },
    Done {
        message: String,
    },
    Ping,
    Stopped {
        reason: String,
        partial_response: Option<String>,
    },
    /// Question pending event for ask_user tool
    QuestionPending {
        question_id: Uuid,
        questions: Vec<Question>,
        created_at: DateTime<Utc>,
    },
    /// Mode changed event for exit_plan_mode tool
    ModeChanged {
        mode: String,
        plan_file: Option<String>,
    },
    /// State changed event for actor state machine transitions
    StateChanged {
        from_state: String,
        to_state: String,
        reason: Option<String>,
    },
}

/// Question definition for ask_user tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub name: String,
    pub question: String,
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<QuestionButton>>,
}

/// Button definition for question UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionButton {
    pub label: String,
    pub value: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}
