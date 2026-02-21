//! Actor state definitions for the ChatActor state machine.
//!
//! This module defines the ActorState enum which represents all possible states
//! of a ChatActor throughout its lifecycle.

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

/// Represents the current state of a ChatActor in its lifecycle.
///
/// # State Transitions
///
/// The state machine enforces strict transition rules:
/// - **Terminal states** (Error, Cancelled, Completed) cannot transition to any other state
/// - Can always transition to Running from any non-terminal state
/// - Running → Idle, Paused, Completed, Error
/// - Idle → Paused
/// - Paused → Idle, Completed
///
/// # Terminal States
///
/// Once an actor enters a terminal state (Error, Cancelled, Completed),
/// it cannot transition to any other state and will eventually be shut down.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString,
)]
pub enum ActorState {
    /// Actor is idle and waiting for user input
    Idle,

    /// Actor is actively processing an interaction
    Running,

    /// Actor has been paused (can be resumed)
    Paused,

    /// Actor encountered an error (terminal state)
    Error,

    /// Actor was cancelled by the user (terminal state)
    Cancelled,

    /// Actor completed naturally (terminal state)
    Completed,
}

impl ActorState {
    /// Returns true if this is a terminal state (cannot transition out).
    ///
    /// Terminal states are: Error, Cancelled, Completed
    ///
    /// # Example
    ///
    /// ```rust
    /// use buildscale::services::chat::state_machine::ActorState;
    ///
    /// assert!(!ActorState::Idle.is_terminal());
    /// assert!(!ActorState::Running.is_terminal());
    /// assert!(ActorState::Error.is_terminal());
    /// assert!(ActorState::Cancelled.is_terminal());
    /// assert!(ActorState::Completed.is_terminal());
    /// ```
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Error | Self::Cancelled | Self::Completed)
    }

    /// Checks if a transition from this state to the target state is allowed.
    ///
    /// This implements the same validation logic as `validate_status_transition`
    /// in the agent_sessions service.
    ///
    /// # Transition Rules
    ///
    /// - Terminal states cannot transition to any state
    /// - Can always transition to Running from any non-terminal state
    /// - Running → Idle, Paused, Completed, Error
    /// - Idle → Paused
    /// - Paused → Idle, Completed
    ///
    /// # Example
    ///
    /// ```rust
    /// use buildscale::services::chat::state_machine::ActorState;
    ///
    /// assert!(ActorState::Idle.can_transition_to(ActorState::Running));
    /// assert!(ActorState::Running.can_transition_to(ActorState::Idle));
    /// assert!(ActorState::Running.can_transition_to(ActorState::Paused));
    /// assert!(ActorState::Paused.can_transition_to(ActorState::Idle));
    /// assert!(!ActorState::Idle.can_transition_to(ActorState::Idle));
    /// assert!(!ActorState::Error.can_transition_to(ActorState::Idle)); // Terminal
    /// ```
    pub fn can_transition_to(self, target: Self) -> bool {
        // Terminal states cannot transition
        if self.is_terminal() {
            return false;
        }

        match (self, target) {
            // Can always transition to running from non-terminal
            (_, Self::Running) => true,

            // Running can go to idle, paused, completed, error, cancelled
            (Self::Running, Self::Idle) => true,
            (Self::Running, Self::Paused) => true,
            (Self::Running, Self::Completed) => true,
            (Self::Running, Self::Error) => true,
            (Self::Running, Self::Cancelled) => true,

            // Idle can go to paused, completed, cancelled
            (Self::Idle, Self::Paused) => true,
            (Self::Idle, Self::Completed) => true,
            (Self::Idle, Self::Cancelled) => true,

            // Paused can go to idle, completed, cancelled
            (Self::Paused, Self::Idle) => true,
            (Self::Paused, Self::Completed) => true,
            (Self::Paused, Self::Cancelled) => true,

            // All other transitions are invalid
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_states() {
        assert!(!ActorState::Idle.is_terminal());
        assert!(!ActorState::Running.is_terminal());
        assert!(!ActorState::Paused.is_terminal());
        assert!(ActorState::Error.is_terminal());
        assert!(ActorState::Cancelled.is_terminal());
        assert!(ActorState::Completed.is_terminal());
    }

    #[test]
    fn test_idle_transitions() {
        let from = ActorState::Idle;
        assert!(from.can_transition_to(ActorState::Running));
        assert!(from.can_transition_to(ActorState::Paused));
        assert!(!from.can_transition_to(ActorState::Idle));
        assert!(!from.can_transition_to(ActorState::Completed));
        assert!(!from.can_transition_to(ActorState::Error));
        assert!(!from.can_transition_to(ActorState::Cancelled));
    }

    #[test]
    fn test_running_transitions() {
        let from = ActorState::Running;
        assert!(from.can_transition_to(ActorState::Idle));
        assert!(from.can_transition_to(ActorState::Paused));
        assert!(from.can_transition_to(ActorState::Completed));
        assert!(from.can_transition_to(ActorState::Error));
        assert!(!from.can_transition_to(ActorState::Running));
        assert!(!from.can_transition_to(ActorState::Cancelled));
    }

    #[test]
    fn test_paused_transitions() {
        let from = ActorState::Paused;
        assert!(from.can_transition_to(ActorState::Idle));
        assert!(from.can_transition_to(ActorState::Completed));
        assert!(from.can_transition_to(ActorState::Running));
        assert!(!from.can_transition_to(ActorState::Paused));
        assert!(!from.can_transition_to(ActorState::Error));
        assert!(!from.can_transition_to(ActorState::Cancelled));
    }

    #[test]
    fn test_terminal_state_no_transitions() {
        for terminal_state in [ActorState::Error, ActorState::Cancelled, ActorState::Completed] {
            for target in [
                ActorState::Idle,
                ActorState::Running,
                ActorState::Paused,
                ActorState::Completed,
                ActorState::Error,
                ActorState::Cancelled,
            ] {
                assert!(
                    !terminal_state.can_transition_to(target),
                    "Terminal state {:?} should not allow transition to {:?}",
                    terminal_state,
                    target
                );
            }
        }
    }
}
