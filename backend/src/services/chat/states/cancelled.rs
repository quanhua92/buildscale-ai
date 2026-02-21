//! Cancelled state handler for ChatActor.
//!
//! The Cancelled state is a terminal state representing when the actor was cancelled by the user.

use crate::error::Result;
use crate::models::agent_session::SessionStatus;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult};
use crate::services::chat::states::{StateContext, StateHandler};

/// Handler for the Cancelled (terminal) state.
///
/// The Cancelled state is terminal - no transitions are allowed.
/// The actor was explicitly cancelled by the user and will be shut down.
#[derive(Debug, Clone)]
pub struct CancelledState;

impl CancelledState {
    /// Creates a new CancelledState handler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CancelledState {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHandler for CancelledState {
    fn state(&self) -> ActorState {
        ActorState::Cancelled
    }

    fn on_enter(&self, _ctx: &mut StateContext) -> Result<Vec<crate::services::chat::state_machine::StateAction>> {
        Ok(vec![
            crate::services::chat::state_machine::StateAction::UpdateSessionStatus(SessionStatus::Cancelled),
        ])
    }

    fn handle_event(&self, _event: ActorEvent, _ctx: &mut StateContext) -> Result<EventResult> {
        // Terminal state - reject all events
        Ok(EventResult::no_change())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancelled_state() {
        let handler = CancelledState::new();
        assert_eq!(handler.state(), ActorState::Cancelled);
    }
}
