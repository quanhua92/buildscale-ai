//! State machine utility functions for ChatActor
//!
//! This module contains standalone helper functions for state machine operations
//! that don't require access to ChatActor's internal state.

use crate::services::chat::registry::AgentCommand;
use crate::services::chat::state_machine::ActorEvent;

/// Convert an AgentCommand to an ActorEvent.
///
/// This bridges the existing command system with the new state machine.
pub fn command_to_event(command: &AgentCommand) -> Option<ActorEvent> {
    match command {
        AgentCommand::ProcessInteraction { user_id } => {
            Some(ActorEvent::ProcessInteraction { user_id: *user_id })
        }
        AgentCommand::Pause { .. } => {
            Some(ActorEvent::Pause { reason: None })
        }
        AgentCommand::Cancel { reason, .. } => {
            Some(ActorEvent::Cancel { reason: reason.clone() })
        }
        AgentCommand::Ping => {
            Some(ActorEvent::Ping)
        }
        AgentCommand::Shutdown => {
            Some(ActorEvent::Shutdown)
        }
    }
}
