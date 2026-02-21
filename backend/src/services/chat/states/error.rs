//! Error state handler for ChatActor.
//!
//! The Error state is a terminal state representing when the actor encountered an error.

use crate::error::Result;
use crate::models::agent_session::SessionStatus;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::chat::states::{StateContext, StateHandler};

/// Handler for the Error (terminal) state.
///
/// The Error state is terminal - no transitions are allowed.
/// The actor will eventually be shut down.
#[derive(Debug, Clone)]
pub struct ErrorState;

impl ErrorState {
    /// Creates a new ErrorState handler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ErrorState {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHandler for ErrorState {
    fn state(&self) -> ActorState {
        ActorState::Error
    }

    fn on_enter(&self, _ctx: &mut StateContext) -> Result<Vec<StateAction>> {
        Ok(vec![
            StateAction::UpdateSessionStatus(SessionStatus::Error),
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
    fn test_error_state() {
        let handler = ErrorState::new();
        assert_eq!(handler.state(), ActorState::Error);
    }
}
