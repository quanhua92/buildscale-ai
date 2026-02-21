//! Completed state handler for ChatActor.
//!
//! The Completed state is a terminal state representing when the actor completed naturally.

use crate::error::Result;
use crate::models::agent_session::SessionStatus;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::chat::states::{StateContext, StateHandler};

/// Handler for the Completed (terminal) state.
///
/// The Completed state is terminal - no transitions are allowed.
/// The actor completed naturally (e.g., via inactivity timeout) and will be shut down.
#[derive(Debug, Clone)]
pub struct CompletedState;

impl CompletedState {
    /// Creates a new CompletedState handler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CompletedState {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHandler for CompletedState {
    fn state(&self) -> ActorState {
        ActorState::Completed
    }

    fn on_enter(&self, _ctx: &mut StateContext) -> Result<Vec<StateAction>> {
        Ok(vec![
            StateAction::UpdateSessionStatus(SessionStatus::Completed),
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
    fn test_completed_state() {
        let handler = CompletedState::new();
        assert_eq!(handler.state(), ActorState::Completed);
    }
}
