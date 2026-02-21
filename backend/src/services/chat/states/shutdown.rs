//! Shutdown state handler for ChatActor.
//!
//! The Shutdown state represents when the actor is gracefully shutting down.

use crate::error::Result;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::chat::states::{StateContext, StateHandler};

/// Handler for the Shutdown state.
///
/// The Shutdown state is entered when the actor receives a shutdown command.
/// This is a transient state that leads to actor termination.
#[derive(Debug, Clone)]
pub struct ShutdownState;

impl ShutdownState {
    /// Creates a new ShutdownState handler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShutdownState {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHandler for ShutdownState {
    fn state(&self) -> ActorState {
        ActorState::Idle // Represents "shutting down" internally
    }

    fn on_enter(&self, _ctx: &mut StateContext) -> Result<Vec<StateAction>> {
        Ok(vec![
            StateAction::ShutdownActor,
        ])
    }

    fn handle_event(&self, _event: ActorEvent, _ctx: &mut StateContext) -> Result<EventResult> {
        // In shutdown, reject all events
        Ok(EventResult::no_change())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_state() {
        let handler = ShutdownState::new();
        // Shutdown state internally uses Idle as its representation
        assert_eq!(handler.state(), ActorState::Idle);
    }
}
