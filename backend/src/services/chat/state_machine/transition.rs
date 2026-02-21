//! State transition definitions and validation for the ChatActor state machine.
//!
//! This module defines the Transition type, TransitionError, and TransitionTable
//! which manage valid state transitions based on events.

use super::{event::ActorEvent, state::ActorState};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt;

/// A state transition from one state to another triggered by an event.
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: ActorState,
    pub event: ActorEvent,
    pub to: ActorState,
    pub timestamp: DateTime<Utc>,
}

impl Transition {
    /// Creates a new transition record.
    pub fn new(from: ActorState, event: ActorEvent, to: ActorState) -> Self {
        Self {
            from,
            event,
            to,
            timestamp: Utc::now(),
        }
    }
}

/// Errors that can occur during state transitions.
#[derive(Debug, Clone)]
pub enum TransitionError {
    /// The requested transition is not valid for the current state and event.
    InvalidTransition {
        from: ActorState,
        event: ActorEvent,
        attempted: ActorState,
    },

    /// The current state is terminal and cannot process events.
    TerminalState {
        state: ActorState,
        attempted_event: ActorEvent,
    },
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, event, attempted } => write!(
                f,
                "Invalid transition from {:?} to {:?} on event {:?}",
                from, attempted, event
            ),
            Self::TerminalState { state, attempted_event } => write!(
                f,
                "Cannot process event {:?} in terminal state {:?}",
                attempted_event, state
            ),
        }
    }
}

impl std::error::Error for TransitionError {}

/// A table of valid state transitions indexed by (current_state, event).
///
/// The transition table validates that state transitions follow the defined rules.
pub struct TransitionTable {
    transitions: HashMap<(ActorState, String), ActorState>,
}

impl TransitionTable {
    /// Creates a new transition table with default transitions.
    pub fn new() -> Self {
        let mut table = Self {
            transitions: HashMap::new(),
        };
        table.initialize_default_transitions();
        table
    }

    /// Initializes the default state transition rules.
    ///
    /// These rules match the validation logic in `validate_status_transition`
    /// from the agent_sessions service.
    fn initialize_default_transitions(&mut self) {
        // From Idle
        self.insert(ActorState::Idle, "process_interaction", ActorState::Running);
        self.insert(ActorState::Idle, "pause", ActorState::Paused);
        self.insert(ActorState::Idle, "inactivity_timeout", ActorState::Completed);
        self.insert(ActorState::Idle, "cancel", ActorState::Cancelled);

        // From Running
        self.insert(
            ActorState::Running,
            "interaction_complete_success",
            ActorState::Idle,
        );
        self.insert(
            ActorState::Running,
            "interaction_complete_failure",
            ActorState::Error,
        );
        self.insert(ActorState::Running, "pause", ActorState::Paused);
        self.insert(ActorState::Running, "cancel", ActorState::Cancelled);

        // From Paused
        self.insert(ActorState::Paused, "process_interaction", ActorState::Idle);
        self.insert(
            ActorState::Paused,
            "inactivity_timeout",
            ActorState::Completed,
        );
        self.insert(ActorState::Paused, "cancel", ActorState::Cancelled);
    }

    /// Helper to insert a transition into the table.
    fn insert(&mut self, from: ActorState, event: &str, to: ActorState) {
        self.transitions.insert((from, event.to_string()), to);
    }

    /// Gets the target state for a given current state and event type.
    ///
    /// Returns None if the transition is not defined.
    pub fn get_target(&self, from: ActorState, event_type: &str) -> Option<ActorState> {
        self.transitions.get(&(from, event_type.to_string())).copied()
    }

    /// Checks if a transition is valid for the given state and event.
    pub fn is_valid_transition(&self, from: ActorState, event_type: &str) -> bool {
        self.transitions.contains_key(&(from, event_type.to_string()))
    }
}

impl Default for TransitionTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_table_default() {
        let table = TransitionTable::new();

        // Test Idle transitions
        assert_eq!(
            table.get_target(ActorState::Idle, "process_interaction"),
            Some(ActorState::Running)
        );
        assert_eq!(
            table.get_target(ActorState::Idle, "pause"),
            Some(ActorState::Paused)
        );
        assert_eq!(
            table.get_target(ActorState::Idle, "inactivity_timeout"),
            Some(ActorState::Completed)
        );

        // Test Running transitions
        assert_eq!(
            table.get_target(ActorState::Running, "interaction_complete_success"),
            Some(ActorState::Idle)
        );
        assert_eq!(
            table.get_target(ActorState::Running, "pause"),
            Some(ActorState::Paused)
        );
        assert_eq!(
            table.get_target(ActorState::Running, "cancel"),
            Some(ActorState::Cancelled)
        );
    }

    #[test]
    fn test_transition_table_invalid() {
        let table = TransitionTable::new();

        // Invalid transitions should return None
        assert_eq!(
            table.get_target(ActorState::Idle, "shutdown"),
            None
        );
        assert_eq!(
            table.get_target(ActorState::Error, "process_interaction"),
            None
        );
    }

    #[test]
    fn test_is_valid_transition() {
        let table = TransitionTable::new();

        assert!(table.is_valid_transition(ActorState::Idle, "process_interaction"));
        assert!(table.is_valid_transition(ActorState::Running, "pause"));
        assert!(!table.is_valid_transition(ActorState::Idle, "cancel"));
    }

    #[test]
    fn test_transition_display() {
        let err = TransitionError::InvalidTransition {
            from: ActorState::Idle,
            event: ActorEvent::Ping,
            attempted: ActorState::Error,
        };
        assert!(format!("{}", err).contains("Invalid transition"));

        let err = TransitionError::TerminalState {
            state: ActorState::Error,
            attempted_event: ActorEvent::Ping,
        };
        assert!(format!("{}", err).contains("Terminal state"));
    }
}
