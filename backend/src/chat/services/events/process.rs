//! ProcessInteraction event processor for ChatActor.
//!
//! Handles user interaction events which trigger AI processing.

use crate::error::Result;
use crate::agent::models::SessionStatus;
use crate::models::sse::SseEvent;
use crate::chat::services::events::EventProcessor;
use crate::chat::services::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::chat::services::states::StateContext;
use crate::fs::storage::FileStorageService;
use crate::DbPool;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Processor for ProcessInteraction events.
///
/// ProcessInteraction events trigger the main AI interaction flow.
#[allow(dead_code)]
pub struct ProcessInteractionProcessor {
    pool: DbPool,
    storage: Arc<FileStorageService>,
    event_tx: broadcast::Sender<SseEvent>,
    default_persona: String,
    default_context_token_limit: usize,
}

impl ProcessInteractionProcessor {
    /// Creates a new ProcessInteractionProcessor.
    pub fn new(
        pool: DbPool,
        storage: Arc<FileStorageService>,
        event_tx: broadcast::Sender<SseEvent>,
        default_persona: String,
        default_context_token_limit: usize,
    ) -> Self {
        Self {
            pool,
            storage,
            event_tx,
            default_persona,
            default_context_token_limit,
        }
    }
}

impl EventProcessor for ProcessInteractionProcessor {
    fn event_type(&self) -> &'static str {
        "process_interaction"
    }

    fn execute(&self, event: ActorEvent, _ctx: &mut StateContext<'_, '_>) -> Result<EventResult> {
        let ActorEvent::ProcessInteraction { user_id } = event else {
            return Err(crate::error::Error::Internal("Invalid event type for ProcessInteractionProcessor".into()));
        };

        // Match the logic from IdleState handler
        // Transition to Running state with all required actions
        Ok(EventResult::transition_with_reason(
            ActorState::Running,
            "unknown",
            Some("Processing user interaction".to_string()),
        )
        .with_action(StateAction::SetActivelyProcessing(true))
        .with_action(StateAction::UpdateSessionStatus(SessionStatus::Running))
        .with_action(StateAction::StartProcessing { user_id }))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_process_interaction_processor() {
        // This is a placeholder test
        // In real tests, you'd need to set up actual dependencies
    }
}
