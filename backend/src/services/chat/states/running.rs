//! Running state handler for ChatActor.
//!
//! The Running state represents when the actor is actively processing an interaction.

use crate::error::Result;
use crate::models::agent_session::SessionStatus;
use crate::models::sse::SseEvent;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::chat::states::{StateContext, StateHandler};

/// Handler for the Running state.
///
/// In the Running state, the actor is actively processing user input.
/// It can transition to:
/// - Idle (on successful completion)
/// - Error (on failure)
/// - Paused (when Pause is received)
/// - Cancelled (when Cancel is received)
#[derive(Debug, Clone)]
pub struct RunningState;

impl RunningState {
    /// Creates a new RunningState handler.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RunningState {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHandler for RunningState {
    fn state(&self) -> ActorState {
        ActorState::Running
    }

    fn on_enter(&self, _ctx: &mut StateContext) -> Result<Vec<StateAction>> {
        Ok(vec![
            StateAction::UpdateSessionStatus(SessionStatus::Running),
            StateAction::SetActivelyProcessing(true),
        ])
    }

    fn handle_event(&self, event: ActorEvent, _ctx: &mut StateContext) -> Result<EventResult> {
        match event {
            ActorEvent::InteractionComplete { success, error } => {
                if success {
                    // Transition to Idle on success
                    Ok(EventResult::transition_with_reason(
                        SessionStatus::Idle,
                        "running",
                        Some("Interaction completed successfully".to_string()),
                    )
                    .with_action(StateAction::SetActivelyProcessing(false)))
                } else {
                    // Transition to Error on failure
                    let error_msg = error.unwrap_or_else(|| "Unknown error".to_string());
                    Ok(EventResult::transition_with_reason(
                        SessionStatus::Error,
                        "running",
                        Some(format!("Interaction failed: {}", error_msg)),
                    )
                    .with_action(StateAction::SetActivelyProcessing(false)))
                }
            }

            ActorEvent::Pause { reason } => {
                // Pause while running
                let reason_str = reason.unwrap_or_else(|| "Paused during processing".to_string());
                Ok(EventResult::transition_with_reason(
                    SessionStatus::Paused,
                    "running",
                    Some(reason_str),
                )
                .with_action(StateAction::SetActivelyProcessing(false)))
            }

            ActorEvent::Cancel { reason } => {
                // Cancel while running
                Ok(EventResult::transition_with_reason(
                    SessionStatus::Cancelled,
                    "running",
                    Some(reason),
                )
                .with_action(StateAction::SetActivelyProcessing(false)))
            }

            ActorEvent::Ping => {
                // Just acknowledge, no state change
                Ok(EventResult {
                    new_state: None,
                    actions: Vec::new(),
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
    fn test_running_state() {
        let handler = RunningState::new();
        assert_eq!(handler.state(), ActorState::Running);
    }
}
