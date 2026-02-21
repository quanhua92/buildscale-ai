//! Event definitions for the ChatActor state machine.
//!
//! This module defines the ActorEvent enum which represents all events
//! that can trigger state transitions in a ChatActor.

use crate::models::agent_session::SessionStatus;
use crate::models::sse::SseEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::state::ActorState;

/// Events that can trigger state transitions in a ChatActor.
///
/// Each event represents a distinct action or occurrence that may cause
/// the actor to change its state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorEvent {
    /// Process a user interaction (message or tool execution)
    ProcessInteraction { user_id: Uuid },

    /// Pause the current session (optional reason)
    Pause { reason: Option<String> },

    /// Cancel the current session with a reason
    Cancel { reason: String },

    /// Keep-alive ping to prevent inactivity timeout
    Ping,

    /// Graceful shutdown request
    Shutdown,

    /// An interaction has completed (success or failure)
    InteractionComplete {
        success: bool,
        error: Option<String>,
    },

    /// Inactivity timeout has expired
    InactivityTimeout,
}

impl ActorEvent {
    /// Returns a human-readable name for the event type.
    pub fn event_type_name(&self) -> &'static str {
        match self {
            Self::ProcessInteraction { .. } => "process_interaction",
            Self::Pause { .. } => "pause",
            Self::Cancel { .. } => "cancel",
            Self::Ping => "ping",
            Self::Shutdown => "shutdown",
            Self::InteractionComplete { success, .. } => {
                if *success {
                    "interaction_complete_success"
                } else {
                    "interaction_complete_failure"
                }
            }
            Self::InactivityTimeout => "inactivity_timeout",
        }
    }
}

/// Actions that can be executed as part of handling an event.
///
/// These actions are returned by state handlers and executed by the state machine.
#[derive(Debug, Clone)]
pub enum StateAction {
    /// Update the session status in the database
    UpdateSessionStatus(SessionStatus),

    /// Set the actively processing flag
    SetActivelyProcessing(bool),

    /// Emit an SSE event to connected clients
    EmitSse(SseEvent),

    /// Reset the inactivity timeout timer
    ResetInactivityTimer,

    /// Shutdown the actor gracefully
    ShutdownActor,

    /// Save a response to the database
    SaveResponse(String),

    /// Cancel the current interaction token
    CancelInteraction,

    /// Send a success response to a command responder (for Pause/Cancel acknowledgments)
    SendSuccessResponse,

    /// Send a failure response to a command responder
    SendFailureResponse { message: String },
}

/// Result of processing an event in a state handler.
///
/// Contains the new state (if any transition should occur), actions to execute,
/// and SSE events to emit.
#[derive(Debug, Clone)]
pub struct EventResult {
    /// The new state after handling the event (None means no state change)
    pub new_state: Option<ActorState>,

    /// Actions to execute as part of handling this event
    pub actions: Vec<StateAction>,

    /// SSE events to emit to connected clients
    pub emit_sse: Vec<SseEvent>,
}

impl EventResult {
    /// Creates an EventResult with no state change and no actions.
    pub fn no_change() -> Self {
        Self {
            new_state: None,
            actions: Vec::new(),
            emit_sse: Vec::new(),
        }
    }

    /// Creates an EventResult that transitions to a specific state.
    pub fn transition_to(state: ActorState) -> Self {
        Self {
            new_state: Some(state),
            actions: vec![],
            emit_sse: Vec::new(),
        }
    }

    /// Creates an EventResult that transitions to a state with a reason.
    pub fn transition_with_reason(
        state: ActorState,
        from_state: &str,
        reason: Option<String>,
    ) -> Self {
        let sse_event = SseEvent::StateChanged {
            from_state: from_state.to_string(),
            to_state: state.to_string(),
            reason,
        };

        Self {
            new_state: Some(state),
            actions: vec![
                StateAction::EmitSse(sse_event.clone()),
            ],
            emit_sse: vec![sse_event],
        }
    }

    /// Adds an action to this result.
    pub fn with_action(mut self, action: StateAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Adds an SSE event to this result.
    pub fn with_sse(mut self, event: SseEvent) -> Self {
        self.emit_sse.push(event);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_names() {
        assert_eq!(
            ActorEvent::ProcessInteraction {
                user_id: Uuid::new_v4()
            }
            .event_type_name(),
            "process_interaction"
        );
        assert_eq!(
            ActorEvent::Pause {
                reason: Some("test".to_string())
            }
            .event_type_name(),
            "pause"
        );
        assert_eq!(
            ActorEvent::Cancel {
                reason: "test".to_string()
            }
            .event_type_name(),
            "cancel"
        );
        assert_eq!(ActorEvent::Ping.event_type_name(), "ping");
        assert_eq!(ActorEvent::Shutdown.event_type_name(), "shutdown");
    }

    #[test]
    fn test_event_result_no_change() {
        let result = EventResult::no_change();
        assert!(result.new_state.is_none());
        assert!(result.actions.is_empty());
        assert!(result.emit_sse.is_empty());
    }

    #[test]
    fn test_event_result_transition() {
        let result = EventResult::transition_to(ActorState::Running);
        assert_eq!(result.new_state, Some(ActorState::Running));
        assert_eq!(result.actions.len(), 0);
        assert!(result.emit_sse.is_empty());
    }
}
