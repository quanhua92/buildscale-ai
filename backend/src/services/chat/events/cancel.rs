//! Cancel event processor for ChatActor.
//!
//! Handles cancel events for cancelling the actor.

use crate::error::Result;
use crate::models::agent_session::SessionStatus;
use crate::services::chat::events::EventProcessor;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::chat::states::StateContext;

/// Processor for Cancel events.
///
/// Cancel events terminate the actor's processing and move it to a terminal state.
#[derive(Debug, Clone)]
pub struct CancelProcessor;

impl CancelProcessor {
    /// Creates a new CancelProcessor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CancelProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProcessor for CancelProcessor {
    fn event_type(&self) -> &'static str {
        "cancel"
    }

    fn execute(&self, event: ActorEvent, _ctx: &mut StateContext<'_>) -> Result<EventResult> {
        let ActorEvent::Cancel { reason } = event else {
            return Err(crate::error::Error::Internal("Invalid event type for CancelProcessor".into()));
        };

        // Transition to Cancelled terminal state
        Ok(EventResult::transition_with_reason(
            ActorState::Cancelled,
            "unknown",
            Some(reason),
        )
        .with_action(StateAction::UpdateSessionStatus(SessionStatus::Cancelled)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancel_processor() {
        let processor = CancelProcessor::new();
        assert_eq!(processor.event_type(), "cancel");
    }
}
