//! State structures for ChatActor
//!
//! This module contains all state-related structs used by the ChatActor.

use tokio_util::sync::CancellationToken;

/// Current tool execution tracking
/// These fields are always read/written together
pub struct ToolTracking {
    /// Track current tool name for logging when ToolResult arrives
    pub current_tool_name: Option<String>,
    /// Track current tool arguments for logging when ToolResult arrives
    pub current_tool_args: Option<serde_json::Value>,
}

/// Interaction lifecycle management
/// These fields manage the current interaction's lifecycle
pub struct InteractionState {
    /// Cancellation token for the current interaction
    pub current_cancellation_token: Option<CancellationToken>,
    /// Track current model for cancellation metadata
    pub current_model: Option<String>,
    /// Current task description for session tracking
    pub current_task: Option<String>,
    /// Flag to track if the actor is actively processing an interaction
    /// Used to prevent inactivity timeout during long-running tasks
    pub is_actively_processing: bool,
}

/// Consolidated state for ChatActor to reduce lock contention
/// All state that was previously in separate Arc<Mutex<>> fields is now grouped logically
pub struct ChatActorState {
    /// Tool Tracking (always accessed in pairs)
    pub tool_tracking: ToolTracking,
    /// Interaction Lifecycle (independent access)
    pub interaction: InteractionState,
    /// Current reasoning session tracking (for audit trail)
    pub current_reasoning_id: Option<String>,
    /// Buffer for reasoning chunks (aggregated before DB persistence)
    pub reasoning_buffer: Vec<String>,
}

impl ChatActorState {
    pub fn ensure_reasoning_id(&mut self) -> String {
        self.current_reasoning_id
            .get_or_insert_with(|| uuid::Uuid::now_v7().to_string())
            .clone()
    }
}

impl Default for ChatActorState {
    fn default() -> Self {
        Self {
            tool_tracking: ToolTracking {
                current_tool_name: None,
                current_tool_args: None,
            },
            interaction: InteractionState {
                current_cancellation_token: None,
                current_model: None,
                current_task: None,
                is_actively_processing: false,
            },
            current_reasoning_id: None,
            reasoning_buffer: Vec::new(),
        }
    }
}
