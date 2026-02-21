//! ProcessInteraction event processor for ChatActor.
//!
//! Handles user interaction events which trigger AI processing.

use crate::error::Result;
use crate::models::agent_session::SessionStatus;
use crate::models::sse::SseEvent;
use crate::services::chat::events::EventProcessor;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::chat::states::StateContext;
use crate::services::storage::FileStorageService;
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

    fn execute(&self, event: ActorEvent, _ctx: &mut StateContext<'_>) -> Result<EventResult> {
        let ActorEvent::ProcessInteraction { user_id } = event else {
            return Err(crate::error::Error::Internal("Invalid event type for ProcessInteractionProcessor".into()));
        };

        // Transition to Running state
        Ok(EventResult::transition_with_reason(
            ActorState::Running,
            "unknown",
            Some(format!("Processing interaction for user {}", user_id)),
        )
        .with_action(StateAction::SetActivelyProcessing(true))
        .with_action(StateAction::UpdateSessionStatus(SessionStatus::Running)))
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
