//! Event processors for the ChatActor state machine.
//!
//! This module provides event processors that handle specific events
//! in the context of the state machine.

pub mod cancel;
pub mod pause;
pub mod ping;
pub mod process_interaction;
pub mod shutdown;

use crate::error::Result;
use crate::models::sse::SseEvent;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult};
use crate::services::chat::states::StateContext;
use crate::services::storage::FileStorageService;
use crate::DbPool;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Trait for processing events in the ChatActor.
///
/// Event processors handle the specific logic for each event type,
/// such as executing tool calls, updating state, or emitting SSE events.
pub trait EventProcessor: Send + Sync {
    /// Returns the event type this processor handles.
    fn event_type(&self) -> &'static str;

    /// Validates that an event can be processed in the current state.
    fn validate(&self, event: &ActorEvent, current_state: ActorState) -> Result<()> {
        // Default validation: terminal states cannot process events
        if current_state.is_terminal() {
            return Err(crate::error::Error::Conflict(format!(
                "Cannot process event {:?} in terminal state {:?}",
                event, current_state
            )));
        }
        Ok(())
    }

    /// Executes the event processing logic.
    fn execute(&self, event: ActorEvent, ctx: &mut StateContext<'_, '_>) -> Result<EventResult>;
}

/// Registry of event processors.
///
/// Provides access to the appropriate processor for each event type.
pub struct EventProcessorRegistry {
    process_interaction: process_interaction::ProcessInteractionProcessor,
    pause: pause::PauseProcessor,
    cancel: cancel::CancelProcessor,
    ping: ping::PingProcessor,
    shutdown: shutdown::ShutdownProcessor,
}

impl EventProcessorRegistry {
    /// Creates a new event processor registry.
    pub fn new(
        pool: DbPool,
        storage: Arc<FileStorageService>,
        event_tx: broadcast::Sender<SseEvent>,
        default_persona: String,
        default_context_token_limit: usize,
    ) -> Self {
        Self {
            process_interaction: process_interaction::ProcessInteractionProcessor::new(
                pool.clone(),
                storage.clone(),
                event_tx.clone(),
                default_persona.clone(),
                default_context_token_limit,
            ),
            pause: pause::PauseProcessor::new(),
            cancel: cancel::CancelProcessor::new(),
            ping: ping::PingProcessor::new(),
            shutdown: shutdown::ShutdownProcessor::new(),
        }
    }

    /// Gets the processor for the given event type.
    pub fn get_processor(&self, event: &ActorEvent) -> Option<&dyn EventProcessor> {
        match event {
            ActorEvent::ProcessInteraction { .. } => Some(&self.process_interaction),
            ActorEvent::Pause { .. } => Some(&self.pause),
            ActorEvent::Cancel { .. } => Some(&self.cancel),
            ActorEvent::Ping => Some(&self.ping),
            ActorEvent::Shutdown => Some(&self.shutdown),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_event_processor_registry() {
        // This is a placeholder test
        // In real tests, you'd need to set up actual dependencies
    }
}
