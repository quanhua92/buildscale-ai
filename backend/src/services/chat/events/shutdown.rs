//! Shutdown event processor for ChatActor.
//!
//! Handles shutdown events for gracefully shutting down the actor.

use crate::error::Result;
use crate::services::chat::events::EventProcessor;
use crate::services::chat::state_machine::{ActorEvent, EventResult, StateAction};
use crate::services::chat::states::StateContext;

/// Processor for Shutdown events.
///
/// Shutdown events gracefully terminate the actor.
#[derive(Debug, Clone)]
pub struct ShutdownProcessor;

impl ShutdownProcessor {
    /// Creates a new ShutdownProcessor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShutdownProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProcessor for ShutdownProcessor {
    fn event_type(&self) -> &'static str {
        "shutdown"
    }

    fn execute(&self, event: ActorEvent, _ctx: &mut StateContext<'_, '_>) -> Result<EventResult> {
        if !matches!(event, ActorEvent::Shutdown) {
            return Err(crate::error::Error::Internal("Invalid event type for ShutdownProcessor".into()));
        }

        // Initiate shutdown
        Ok(EventResult {
            new_state: None,
            actions: vec![StateAction::ShutdownActor],
            emit_sse: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_processor() {
        let processor = ShutdownProcessor::new();
        assert_eq!(processor.event_type(), "shutdown");
    }
}
