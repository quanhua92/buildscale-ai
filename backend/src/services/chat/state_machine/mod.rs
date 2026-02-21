//! State machine framework for ChatActor.
//!
//! This module provides a finite state machine (FSM) implementation for managing
//! the lifecycle and state transitions of ChatActor instances.
//!
//! # Architecture
//!
//! The state machine is composed of:
//! - **States** ([ActorState]): Idle, Running, Paused, Error, Cancelled, Completed
//! - **Events** ([ActorEvent]): ProcessInteraction, Pause, Cancel, Ping, Shutdown, etc.
//! - **Transitions**: Valid state changes based on events
//! - **Machine** ([StateMachine]): Generic FSM implementation with logging
//!
//! # State Transition Diagram
//!
//! ```text
//!                    ┌─────────────────────────────────────────┐
//!                    │                                         │
//!                    ▼                                         │
//! ┌─────────┐  ProcessInteraction  ┌──────────┐  InteractionComplete  ┌─────────┐
//! │ Created │ ──────────────────>  │  Idle    │ ──────────────────────> │ Running │
//! └─────────┘                     └──────────┘                      └─────────┘
//!                                           │                              │
//!                                           │ Pause                        │
//!                                           ▼                              │
//!                                     ┌──────────┐                 InteractionComplete
//!                                     │  Paused  │ <─────────────────────────────────┐
//!                                     └──────────┶────────────────────────────────────┤
//!                                             │ (resume)                        (success)
//!                                             ▼                                  │
//!                                           ┌──────────┐                         │
//!                                           │  Idle    │ <───────────────────────────┘
//!                                           └──────────┘
//!                                             │
//!                                             │ InactivityTimeout
//!                                             ▼
//!                                       ┌────────────┐
//!                                       │ Completed  │ (terminal)
//!                                       └────────────┘
//!
//!                     ┌─────────────┐
//!                     │   Error     │ (terminal) - from any state on error
//!                     └─────────────┘
//!
//!                     ┌─────────────┐
//!                     │  Cancelled  │ (terminal) - from any state on Cancel
//!                     └─────────────┘
//! ```
//!
//! # Terminal States
//!
//! Once an actor enters a terminal state (Error, Cancelled, Completed), it cannot
//! transition to any other state and will eventually be shut down.
//!
//! # Example
//!
//! ```rust
//! use buildscale::services::chat::state_machine::{StateMachine, ActorState, ActorEvent};
//! use uuid::Uuid;
//!
//! let mut machine = StateMachine::new(ActorState::Idle);
//!
//! // Process an interaction
//! let user_id = Uuid::now_v7();
//! let event = ActorEvent::ProcessInteraction { user_id };
//! let result = machine.handle_event(event)?;
//!
//! assert_eq!(result.new_state, ActorState::Running);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod event;
mod machine;
mod state;
mod transition;

pub use event::{ActorEvent, EventResult, StateAction};
pub use machine::{StateTransition, StateMachine, TransitionLog};
pub use state::ActorState;
pub use transition::{Transition, TransitionError, TransitionTable};

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// Integration test for the complete state machine flow.
    #[test]
    fn test_complete_interaction_flow() {
        let mut machine = StateMachine::new(ActorState::Idle);

        // Start processing
        let event = ActorEvent::ProcessInteraction {
            user_id: Uuid::new_v4(),
        };
        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Running);

        // Complete successfully
        let event = ActorEvent::InteractionComplete {
            success: true,
            error: None,
        };
        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Idle);

        // Verify we can process again
        let event = ActorEvent::ProcessInteraction {
            user_id: Uuid::new_v4(),
        };
        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Running);
    }

    /// Test pause and resume flow.
    #[test]
    fn test_pause_resume_flow() {
        let mut machine = StateMachine::new(ActorState::Running);

        // Pause
        let event = ActorEvent::Pause {
            reason: Some("User paused".to_string()),
        };
        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Paused);

        // Resume (ProcessInteraction from Paused goes to Idle first)
        let event = ActorEvent::ProcessInteraction {
            user_id: Uuid::new_v4(),
        };
        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Idle);
    }

    /// Test error flow.
    #[test]
    fn test_error_flow() {
        let mut machine = StateMachine::new(ActorState::Running);

        // Error during processing
        let event = ActorEvent::InteractionComplete {
            success: false,
            error: Some("AI engine error".to_string()),
        };
        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Error);

        // Terminal state - cannot transition
        let event = ActorEvent::ProcessInteraction {
            user_id: Uuid::new_v4(),
        };
        assert!(machine.handle_event(event).is_err());
    }

    /// Test cancellation flow.
    #[test]
    fn test_cancellation_flow() {
        let mut machine = StateMachine::new(ActorState::Running);

        // User cancels
        let event = ActorEvent::Cancel {
            reason: "User cancelled".to_string(),
        };
        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Cancelled);

        // Terminal state - cannot transition
        let event = ActorEvent::InteractionComplete {
            success: true,
            error: None,
        };
        assert!(machine.handle_event(event).is_err());
    }

    /// Test inactivity timeout flow.
    #[test]
    fn test_inactivity_timeout_flow() {
        let mut machine = StateMachine::new(ActorState::Idle);

        // Inactivity timeout
        let event = ActorEvent::InactivityTimeout;
        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Completed);

        // Terminal state - cannot transition
        let event = ActorEvent::ProcessInteraction {
            user_id: Uuid::new_v4(),
        };
        assert!(machine.handle_event(event).is_err());
    }
}
