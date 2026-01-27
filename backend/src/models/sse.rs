use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}
