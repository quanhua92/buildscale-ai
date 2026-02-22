//! Pause event processor for ChatActor.
//!
//! Handles pause events for temporarily stopping the actor.

use crate::error::Result;
use crate::models::agent_session::SessionStatus;
use crate::services::chat::events::EventProcessor;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::chat::states::StateContext;

/// Processor for Pause events.
///
/// Pause events temporarily stop the actor's processing.
#[derive(Debug, Clone)]
pub struct PauseProcessor;

impl PauseProcessor {
    /// Creates a new PauseProcessor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PauseProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProcessor for PauseProcessor {
    fn event_type(&self) -> &'static str {
        "pause"
    }

    fn execute(&self, event: ActorEvent, _ctx: &mut StateContext<'_, '_>) -> Result<EventResult> {
        let ActorEvent::Pause { reason } = event else {
            return Err(crate::error::Error::Internal("Invalid event type for PauseProcessor".into()));
        };

        let reason_str = reason.unwrap_or_else(|| "Paused".to_string());

        // Transition to Paused state
        Ok(EventResult::transition_with_reason(
            ActorState::Paused,
            "unknown",
            Some(reason_str),
        )
        .with_action(StateAction::UpdateSessionStatus(SessionStatus::Paused)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pause_processor() {
        let processor = PauseProcessor::new();
        assert_eq!(processor.event_type(), "pause");
    }
}
