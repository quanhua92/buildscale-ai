//! Generic state machine implementation for ChatActor.
//!
//! This module provides the StateMachine struct that manages state transitions,
//! event logging, and transition validation.

use super::{event::ActorEvent, state::ActorState, transition::TransitionError, TransitionTable};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;

/// A log entry recording a state transition.
#[derive(Debug, Clone)]
pub struct TransitionLog<S, E> {
    pub from: S,
    pub event: E,
    pub to: S,
    pub timestamp: DateTime<Utc>,
}

impl<S, E> TransitionLog<S, E>
where
    S: std::fmt::Debug,
    E: std::fmt::Debug,
{
    /// Creates a new transition log entry.
    pub fn new(from: S, event: E, to: S) -> Self {
        Self {
            from,
            event,
            to,
            timestamp: Utc::now(),
        }
    }
}

/// The result of a state transition.
#[derive(Debug)]
pub struct StateTransition<S> {
    /// The new state after the transition
    pub new_state: S,
    /// Whether the state actually changed (some events may not cause changes)
    pub state_changed: bool,
}

/// A generic state machine that manages state transitions based on events.
///
/// The StateMachine validates all transitions against its transition table
/// and maintains a log of all transitions for debugging and auditing.
pub struct StateMachine {
    current_state: ActorState,
    transition_table: TransitionTable,
    event_log: VecDeque<TransitionLog<ActorState, ActorEvent>>,
    max_log_size: usize,
}

impl StateMachine {
    /// Creates a new state machine with the given initial state.
    pub fn new(initial_state: ActorState) -> Self {
        Self {
            current_state: initial_state,
            transition_table: TransitionTable::new(),
            event_log: VecDeque::with_capacity(100),
            max_log_size: 100,
        }
    }

    /// Returns the current state of the machine.
    pub fn current_state(&self) -> ActorState {
        self.current_state
    }

    /// Returns a reference to the transition table.
    pub fn transition_table(&self) -> &TransitionTable {
        &self.transition_table
    }

    /// Returns the event log.
    pub fn event_log(&self) -> &VecDeque<TransitionLog<ActorState, ActorEvent>> {
        &self.event_log
    }

    /// Handles an event and returns the resulting state transition.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The current state is terminal and cannot process events
    /// - The event does not define a valid transition from the current state
    pub fn handle_event(&mut self, event: ActorEvent) -> Result<StateTransition<ActorState>, TransitionError> {
        // Check if current state is terminal
        if self.current_state.is_terminal() {
            return Err(TransitionError::TerminalState {
                state: self.current_state,
                attempted_event: event,
            });
        }

        let event_type = event.event_type_name();
        let from_state = self.current_state;

        // Get the target state for this transition
        let target_state = self
            .transition_table
            .get_target(from_state, event_type)
            .ok_or_else(|| TransitionError::InvalidTransition {
                from: from_state,
                event: event.clone(),
                attempted: from_state, // We don't know the attempted state yet
            })?;

        // Check if the transition is valid using the state's own validation
        if !from_state.can_transition_to(target_state) {
            return Err(TransitionError::InvalidTransition {
                from: from_state,
                event: event.clone(),
                attempted: target_state,
            });
        }

        // Perform the transition
        let state_changed = target_state != self.current_state;
        self.current_state = target_state;

        // Log the transition
        let log_entry = TransitionLog::new(from_state, event, target_state);
        self.event_log.push_back(log_entry);

        // Trim log if it exceeds max size
        while self.event_log.len() > self.max_log_size {
            self.event_log.pop_front();
        }

        Ok(StateTransition {
            new_state: target_state,
            state_changed,
        })
    }

    /// Forces a state transition without validation (use with caution).
    ///
    /// This should only be used for recovery scenarios or testing.
    pub fn force_transition(&mut self, new_state: ActorState) {
        let _from_state = self.current_state;
        self.current_state = new_state;
    }

    /// Returns the number of transitions logged.
    pub fn log_size(&self) -> usize {
        self.event_log.len()
    }

    /// Clears the event log.
    pub fn clear_log(&mut self) {
        self.event_log.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_state_machine_initialization() {
        let machine = StateMachine::new(ActorState::Idle);
        assert_eq!(machine.current_state(), ActorState::Idle);
        assert_eq!(machine.log_size(), 0);
    }

    #[test]
    fn test_idle_to_running_on_process_interaction() {
        let mut machine = StateMachine::new(ActorState::Idle);
        let event = ActorEvent::ProcessInteraction {
            user_id: Uuid::new_v4(),
        };

        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Running);
        assert!(result.state_changed);
        assert_eq!(machine.current_state(), ActorState::Running);
        assert_eq!(machine.log_size(), 1);
    }

    #[test]
    fn test_running_to_idle_on_complete() {
        let mut machine = StateMachine::new(ActorState::Running);
        let event = ActorEvent::InteractionComplete {
            success: true,
            error: None,
        };

        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Idle);
        assert!(result.state_changed);
    }

    #[test]
    fn test_running_to_error_on_failure() {
        let mut machine = StateMachine::new(ActorState::Running);
        let event = ActorEvent::InteractionComplete {
            success: false,
            error: Some("Test error".to_string()),
        };

        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Error);
        assert!(result.state_changed);
    }

    #[test]
    fn test_running_to_paused() {
        let mut machine = StateMachine::new(ActorState::Running);
        let event = ActorEvent::Pause {
            reason: Some("User pause".to_string()),
        };

        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Paused);
        assert!(result.state_changed);
    }

    #[test]
    fn test_idle_to_paused() {
        let mut machine = StateMachine::new(ActorState::Idle);
        let event = ActorEvent::Pause {
            reason: None,
        };

        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Paused);
        assert!(result.state_changed);
    }

    #[test]
    fn test_paused_to_idle_on_interaction() {
        let mut machine = StateMachine::new(ActorState::Paused);
        let event = ActorEvent::ProcessInteraction {
            user_id: Uuid::new_v4(),
        };

        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Idle);
        assert!(result.state_changed);
    }

    #[test]
    fn test_idle_to_completed_on_timeout() {
        let mut machine = StateMachine::new(ActorState::Idle);
        let event = ActorEvent::InactivityTimeout;

        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Completed);
        assert!(result.state_changed);
    }

    #[test]
    fn test_running_to_cancelled() {
        let mut machine = StateMachine::new(ActorState::Running);
        let event = ActorEvent::Cancel {
            reason: "User cancelled".to_string(),
        };

        let result = machine.handle_event(event).unwrap();
        assert_eq!(result.new_state, ActorState::Cancelled);
        assert!(result.state_changed);
    }

    #[test]
    fn test_terminal_states_block_transitions() {
        for terminal_state in [ActorState::Error, ActorState::Cancelled, ActorState::Completed] {
            let mut machine = StateMachine::new(terminal_state);

            // Try various events
            let events = vec![
                ActorEvent::ProcessInteraction {
                    user_id: Uuid::new_v4(),
                },
                ActorEvent::Pause { reason: None },
                ActorEvent::Ping,
            ];

            for event in events {
                let result = machine.handle_event(event.clone());
                assert!(
                    matches!(result, Err(TransitionError::TerminalState { .. })),
                    "Terminal state {:?} should block event {:?}",
                    terminal_state,
                    event
                );
            }
        }
    }

    #[test]
    fn test_invalid_transition() {
        let mut machine = StateMachine::new(ActorState::Idle);

        // Ping doesn't define a transition from Idle
        let event = ActorEvent::Ping;
        let result = machine.handle_event(event);

        // Should fail with InvalidTransition
        assert!(matches!(result, Err(TransitionError::InvalidTransition { .. })));
    }

    #[test]
    fn test_event_log_trimming() {
        let mut machine = StateMachine::new(ActorState::Idle);

        // Set a small max log size for testing
        machine.max_log_size = 5;

        // Generate more transitions than max_log_size
        for _ in 0..10 {
            let event = ActorEvent::ProcessInteraction {
                user_id: Uuid::new_v4(),
            };
            // Reset to idle for next iteration
            if machine.current_state() == ActorState::Running {
                machine.force_transition(ActorState::Idle);
            }
            let _ = machine.handle_event(event);
        }

        // Log should be trimmed to max_log_size
        assert!(machine.log_size() <= machine.max_log_size);
    }

    #[test]
    fn test_force_transition() {
        let mut machine = StateMachine::new(ActorState::Idle);

        // Force transition without validation
        machine.force_transition(ActorState::Error);

        assert_eq!(machine.current_state(), ActorState::Error);
        assert_eq!(machine.log_size(), 0); // Force doesn't log
    }
}
