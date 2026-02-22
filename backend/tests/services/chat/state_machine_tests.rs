//! Unit tests for the ChatActor state machine.
//!
//! These tests verify the state transition logic matches the rules
//! defined in `validate_status_transition` from agent_sessions service.

use buildscale::services::chat::state_machine::{
    ActorEvent, ActorState, StateMachine, TransitionError,
};
use uuid::Uuid;

#[tokio::test]
async fn test_idle_to_running_on_process_interaction() {
    let mut machine = StateMachine::new(ActorState::Idle);
    let event = ActorEvent::ProcessInteraction {
        user_id: Uuid::new_v4(),
    };

    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Running);
    assert!(result.state_changed);
    assert_eq!(machine.current_state(), ActorState::Running);
}

#[tokio::test]
async fn test_running_to_idle_on_complete_success() {
    let mut machine = StateMachine::new(ActorState::Running);
    let event = ActorEvent::InteractionComplete {
        success: true,
        error: None,
    };

    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Idle);
    assert!(result.state_changed);
}

#[tokio::test]
async fn test_running_to_idle_on_complete_failure() {
    let mut machine = StateMachine::new(ActorState::Running);
    let event = ActorEvent::InteractionComplete {
        success: false,
        error: Some("Test error".to_string()),
    };

    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Idle);
    assert!(result.state_changed);
}

#[tokio::test]
async fn test_running_to_paused() {
    let mut machine = StateMachine::new(ActorState::Running);
    let event = ActorEvent::Pause {
        reason: Some("User pause".to_string()),
    };

    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Paused);
    assert!(result.state_changed);
}

#[tokio::test]
async fn test_running_to_cancelled() {
    let mut machine = StateMachine::new(ActorState::Running);
    let event = ActorEvent::Cancel {
        reason: "User cancelled".to_string(),
    };

    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Cancelled);
    assert!(result.state_changed);
}

#[tokio::test]
async fn test_idle_to_paused() {
    let mut machine = StateMachine::new(ActorState::Idle);
    let event = ActorEvent::Pause {
        reason: None,
    };

    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Paused);
    assert!(result.state_changed);
}

#[tokio::test]
async fn test_paused_to_idle_on_interaction() {
    let mut machine = StateMachine::new(ActorState::Paused);
    let event = ActorEvent::ProcessInteraction {
        user_id: Uuid::new_v4(),
    };

    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Idle);
    assert!(result.state_changed);
}

#[tokio::test]
async fn test_idle_to_completed_on_timeout() {
    let mut machine = StateMachine::new(ActorState::Idle);
    let event = ActorEvent::InactivityTimeout;

    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Completed);
    assert!(result.state_changed);
}

#[tokio::test]
async fn test_paused_to_completed_on_timeout() {
    let mut machine = StateMachine::new(ActorState::Paused);
    let event = ActorEvent::InactivityTimeout;

    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Completed);
    assert!(result.state_changed);
}

#[tokio::test]
async fn test_terminal_states_block_transitions() {
    for terminal_state in [ActorState::Error, ActorState::Cancelled, ActorState::Completed] {
        let mut machine = StateMachine::new(terminal_state);

        // Try various events
        let events = vec![
            ActorEvent::ProcessInteraction {
                user_id: Uuid::new_v4(),
            },
            ActorEvent::Pause { reason: None },
            ActorEvent::Ping,
            ActorEvent::InactivityTimeout,
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

#[tokio::test]
async fn test_invalid_transition() {
    let mut machine = StateMachine::new(ActorState::Idle);

    // Ping doesn't define a transition from Idle
    let event = ActorEvent::Ping;
    let result = machine.handle_event(event);

    // Should fail with InvalidTransition
    assert!(matches!(result, Err(TransitionError::InvalidTransition { .. })));
}

#[tokio::test]
async fn test_ping_from_idle_is_invalid() {
    let mut machine = StateMachine::new(ActorState::Idle);
    let event = ActorEvent::Ping;

    let result = machine.handle_event(event);
    assert!(matches!(result, Err(TransitionError::InvalidTransition { .. })));
}

#[tokio::test]
async fn test_state_does_not_change_on_no_transition() {
    let mut machine = StateMachine::new(ActorState::Idle);
    let initial_state = machine.current_state();

    // Try an invalid event
    let event = ActorEvent::Ping;
    let _ = machine.handle_event(event);

    // State should remain unchanged
    assert_eq!(machine.current_state(), initial_state);
}

#[tokio::test]
async fn test_multiple_transitions_in_sequence() {
    let mut machine = StateMachine::new(ActorState::Idle);

    // Idle -> Running
    let event1 = ActorEvent::ProcessInteraction {
        user_id: Uuid::new_v4(),
    };
    let result1 = machine.handle_event(event1).unwrap();
    assert_eq!(result1.new_state, ActorState::Running);

    // Running -> Idle (success)
    let event2 = ActorEvent::InteractionComplete {
        success: true,
        error: None,
    };
    let result2 = machine.handle_event(event2).unwrap();
    assert_eq!(result2.new_state, ActorState::Idle);

    // Idle -> Running again
    let event3 = ActorEvent::ProcessInteraction {
        user_id: Uuid::new_v4(),
    };
    let result3 = machine.handle_event(event3).unwrap();
    assert_eq!(result3.new_state, ActorState::Running);
}

#[tokio::test]
async fn test_pause_resume_flow() {
    let mut machine = StateMachine::new(ActorState::Running);

    // Pause
    let event1 = ActorEvent::Pause {
        reason: Some("User paused".to_string()),
    };
    let result1 = machine.handle_event(event1).unwrap();
    assert_eq!(result1.new_state, ActorState::Paused);

    // Resume (ProcessInteraction from Paused goes to Idle first)
    let event2 = ActorEvent::ProcessInteraction {
        user_id: Uuid::new_v4(),
    };
    let result2 = machine.handle_event(event2).unwrap();
    assert_eq!(result2.new_state, ActorState::Idle);
}

#[tokio::test]
async fn test_error_flow() {
    let mut machine = StateMachine::new(ActorState::Running);

    // Transient error during processing (e.g., AI timeout)
    let event = ActorEvent::InteractionComplete {
        success: false,
        error: Some("AI engine error".to_string()),
    };
    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Idle);

    // Should be able to process another interaction (not terminal)
    let event2 = ActorEvent::ProcessInteraction {
        user_id: Uuid::new_v4(),
    };
    let result2 = machine.handle_event(event2).unwrap();
    assert_eq!(result2.new_state, ActorState::Running);
}

#[tokio::test]
async fn test_cancellation_flow() {
    let mut machine = StateMachine::new(ActorState::Running);

    // User cancels
    let event = ActorEvent::Cancel {
        reason: "User cancelled".to_string(),
    };
    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Cancelled);

    // Terminal state - cannot transition
    let event2 = ActorEvent::InteractionComplete {
        success: true,
        error: None,
    };
    assert!(machine.handle_event(event2).is_err());
}

#[tokio::test]
async fn test_inactivity_timeout_flow() {
    let mut machine = StateMachine::new(ActorState::Idle);

    // Inactivity timeout
    let event = ActorEvent::InactivityTimeout;
    let result = machine.handle_event(event).unwrap();
    assert_eq!(result.new_state, ActorState::Completed);

    // Terminal state - cannot transition
    let event2 = ActorEvent::ProcessInteraction {
        user_id: Uuid::new_v4(),
    };
    assert!(machine.handle_event(event2).is_err());
}

#[tokio::test]
async fn test_event_log_recording() {
    let mut machine = StateMachine::new(ActorState::Idle);

    // Perform a few transitions
    let event1 = ActorEvent::ProcessInteraction {
        user_id: Uuid::new_v4(),
    };
    machine.handle_event(event1).unwrap();

    let event2 = ActorEvent::InteractionComplete {
        success: true,
        error: None,
    };
    machine.handle_event(event2).unwrap();

    // Check that transitions were logged
    assert_eq!(machine.log_size(), 2);

    let log = machine.event_log();
    assert_eq!(log[0].from, ActorState::Idle);
    assert_eq!(log[0].to, ActorState::Running);
    assert_eq!(log[1].from, ActorState::Running);
    assert_eq!(log[1].to, ActorState::Idle);
}
