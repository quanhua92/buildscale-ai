//! Ping event processor for ChatActor.
//!
//! Handles ping events for keep-alive functionality.

use crate::error::Result;
use crate::models::sse::SseEvent;
use crate::services::chat::events::EventProcessor;
use crate::services::chat::state_machine::{ActorEvent, EventResult, StateAction};
use crate::services::chat::states::StateContext;

/// Processor for Ping events.
///
/// Ping events are used for keep-alive functionality to prevent
/// inactivity timeout.
#[derive(Debug, Clone)]
pub struct PingProcessor;

impl PingProcessor {
    /// Creates a new PingProcessor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PingProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProcessor for PingProcessor {
    fn event_type(&self) -> &'static str {
        "ping"
    }

    fn execute(&self, _event: ActorEvent, _ctx: &mut StateContext<'_, '_>) -> Result<EventResult> {
        // Ping event: acknowledge and reset inactivity timer
        Ok(EventResult {
            new_state: None,
            actions: vec![StateAction::ResetInactivityTimer],
            emit_sse: vec![SseEvent::Ping],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_processor() {
        let processor = PingProcessor::new();
        assert_eq!(processor.event_type(), "ping");
    }
}
