//! Paused state handler for ChatActor.
//!
//! The Paused state represents when the actor has been temporarily paused.

use crate::error::Result;
use crate::models::agent_session::SessionStatus;
use crate::models::sse::SseEvent;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::chat::states::{StateContext, StateHandler};

/// Handler for the Paused state.
///
/// In the Paused state, the actor has been temporarily stopped.
/// It can transition to:
/// - Idle (when ProcessInteraction is received - resumes to idle first)
/// - Completed (on InactivityTimeout)
#[derive(Debug, Clone)]
pub struct PausedState;

impl PausedState {
    /// Creates a new PausedState handler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PausedState {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHandler for PausedState {
    fn state(&self) -> ActorState {
        ActorState::Paused
    }

    fn on_enter(&self, _ctx: &mut StateContext) -> Result<Vec<StateAction>> {
        Ok(vec![
            StateAction::UpdateSessionStatus(SessionStatus::Paused),
            StateAction::SetActivelyProcessing(false),
        ])
    }

    fn handle_event(&self, event: ActorEvent, _ctx: &mut StateContext) -> Result<EventResult> {
        match event {
            ActorEvent::ProcessInteraction { user_id: _ } => {
                // Resume to Idle first, then will transition to Running
                Ok(EventResult::transition_with_reason(
                    ActorState::Idle,
                    "paused",
                    Some("Resuming from pause".to_string()),
                )
                .with_action(StateAction::UpdateSessionStatus(SessionStatus::Idle)))
            }

            ActorEvent::Pause { reason: _ } => {
                // Already paused, emit a ping to acknowledge
                Ok(EventResult {
                    new_state: None,
                    actions: Vec::new(),
                    emit_sse: vec![SseEvent::Ping],
                })
            }

            ActorEvent::InactivityTimeout => {
                // Transition to Completed (terminal)
                Ok(EventResult::transition_with_reason(
                    ActorState::Completed,
                    "paused",
                    Some("Inactivity timeout while paused".to_string()),
                )
                .with_action(StateAction::ShutdownActor)
                .with_action(StateAction::UpdateSessionStatus(SessionStatus::Completed)))
            }

            ActorEvent::Ping => {
                // Acknowledge ping
                Ok(EventResult {
                    new_state: None,
                    actions: vec![StateAction::ResetInactivityTimer],
                    emit_sse: vec![SseEvent::Ping],
                })
            }

            _ => Ok(EventResult::no_change()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paused_state() {
        let handler = PausedState::new();
        assert_eq!(handler.state(), ActorState::Paused);
    }
}
