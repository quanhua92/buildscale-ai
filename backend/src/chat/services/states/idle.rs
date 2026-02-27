//! Idle state handler for ChatActor.
//!
//! The Idle state represents when the actor is waiting for user input.

use crate::error::Result;
use crate::models::agent_session::SessionStatus;
use crate::models::sse::SseEvent;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::chat::states::{StateContext, StateHandler};

/// Handler for the Idle state.
///
/// In the Idle state, the actor is waiting for user input or events.
/// It can transition to:
/// - Running (when ProcessInteraction is received)
/// - Paused (when Pause is received)
/// - Completed (on InactivityTimeout)
#[derive(Debug, Clone)]
pub struct IdleState;

impl IdleState {
    /// Creates a new IdleState handler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for IdleState {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHandler for IdleState {
    fn state(&self) -> ActorState {
        ActorState::Idle
    }

    fn on_enter(&self, _ctx: &mut StateContext) -> Result<Vec<StateAction>> {
        Ok(vec![
            StateAction::UpdateSessionStatus(SessionStatus::Idle),
            StateAction::ResetInactivityTimer,
        ])
    }

    fn handle_event(&self, event: ActorEvent, ctx: &mut StateContext) -> Result<EventResult> {
        // Note: shared_state is currently None as we migrate to the new architecture
        // State handlers can access it via ctx.shared_state when needed
        let _shared_state = ctx.shared_state; // Acknowledge the field exists

        match event {
            ActorEvent::ProcessInteraction { user_id } => {
                // Transition to Running and trigger AI processing
                Ok(EventResult::transition_with_reason(
                    ActorState::Running,
                    "idle",
                    Some("Processing user interaction".to_string()),
                )
                .with_action(StateAction::SetActivelyProcessing(true))
                .with_action(StateAction::UpdateSessionStatus(SessionStatus::Running))
                .with_action(StateAction::StartProcessing { user_id }))
            }

            ActorEvent::Pause { reason } => {
                // Already idle, can pause
                let reason_str = reason.unwrap_or_else(|| "Paused while idle".to_string());
                Ok(EventResult::transition_with_reason(
                    ActorState::Paused,
                    "idle",
                    Some(reason_str),
                )
                .with_action(StateAction::UpdateSessionStatus(SessionStatus::Paused))
                .with_action(StateAction::CancelInteraction)
                .with_action(StateAction::SendSuccessResponse))
            }

            ActorEvent::Cancel { reason } => {
                // Cancel while idle
                Ok(EventResult::transition_with_reason(
                    ActorState::Cancelled,
                    "idle",
                    Some(reason),
                )
                .with_action(StateAction::UpdateSessionStatus(SessionStatus::Cancelled))
                .with_action(StateAction::SendSuccessResponse))
            }

            ActorEvent::InactivityTimeout => {
                // Transition to Completed (terminal)
                Ok(EventResult::transition_with_reason(
                    ActorState::Completed,
                    "idle",
                    Some("Inactivity timeout - session completed".to_string()),
                )
                .with_action(StateAction::ShutdownActor)
                .with_action(StateAction::UpdateSessionStatus(SessionStatus::Completed)))
            }

            ActorEvent::Ping => {
                // Just reset the inactivity timer, no state change
                Ok(EventResult {
                    new_state: None,
                    actions: vec![StateAction::ResetInactivityTimer],
                    emit_sse: vec![SseEvent::Ping],
                })
            }

            ActorEvent::Shutdown => {
                // Transition to Completed (terminal)
                Ok(EventResult::transition_with_reason(
                    ActorState::Completed,
                    "idle",
                    Some("Shutdown requested".to_string()),
                )
                .with_action(StateAction::ShutdownActor)
                .with_action(StateAction::UpdateSessionStatus(SessionStatus::Completed)))
            }

            _ => Ok(EventResult::no_change()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_state() {
        let handler = IdleState::new();
        assert_eq!(handler.state(), ActorState::Idle);
    }
}
