//! Shutdown event processor for ChatActor.
//!
//! Handles shutdown events for gracefully shutting down the actor.

use crate::error::Result;
use crate::agent::models::SessionStatus;
use crate::chat::services::events::EventProcessor;
use crate::chat::services::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::chat::services::states::StateContext;

/// Processor for Shutdown events.
///
/// Shutdown events gracefully terminate the actor.
#[derive(Debug, Clone)]
pub struct ShutdownProcessor;

impl ShutdownProcessor {
    /// Creates a new ShutdownProcessor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShutdownProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProcessor for ShutdownProcessor {
    fn event_type(&self) -> &'static str {
        "shutdown"
    }

    fn execute(&self, event: ActorEvent, _ctx: &mut StateContext<'_, '_>) -> Result<EventResult> {
        if !matches!(event, ActorEvent::Shutdown) {
            return Err(crate::error::Error::Internal("Invalid event type for ShutdownProcessor".into()));
        }

        // Match the logic from IdleState handler
        // Transition to Completed terminal state and shutdown
        Ok(EventResult::transition_with_reason(
            ActorState::Completed,
            "unknown",
            Some("Shutdown requested".to_string()),
        )
        .with_action(StateAction::ShutdownActor)
        .with_action(StateAction::UpdateSessionStatus(SessionStatus::Completed)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_processor() {
        let processor = ShutdownProcessor::new();
        assert_eq!(processor.event_type(), "shutdown");
    }
}
