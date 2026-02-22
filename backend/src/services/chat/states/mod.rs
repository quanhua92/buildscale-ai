//! State-specific handlers for the ChatActor state machine.
//!
//! This module provides implementations of the StateHandler trait for each
//! state in the ChatActor lifecycle. Each handler is responsible for:
//!
//! - Defining behavior when entering the state (on_enter)
//! - Defining behavior when exiting the state (on_exit)
//! - Handling events specific to that state (handle_event)

use crate::error::Result;
use crate::models::sse::SseEvent;
use crate::services::chat::state_machine::{ActorEvent, ActorState, EventResult, StateAction};
use crate::services::storage::FileStorageService;
use crate::DbPool;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, Mutex};
use uuid::Uuid;

pub mod cancelled;
pub mod completed;
pub mod error;
pub mod idle;
pub mod paused;
pub mod running;
pub mod shutdown;

/// Trait for handling state-specific behavior in the ChatActor.
///
/// Each state implements this trait to define its behavior on entry,
/// exit, and when receiving events.
pub trait StateHandler: Send + Sync {
    /// Returns the ActorState this handler is responsible for.
    fn state(&self) -> ActorState;

    /// Called when entering this state.
    ///
    /// Returns a list of actions to execute during the state transition.
    fn on_enter(&self, _ctx: &mut StateContext<'_, '_>) -> Result<Vec<StateAction>> {
        Ok(Vec::new())
    }

    /// Called when exiting this state.
    ///
    /// Returns a list of actions to execute during the state transition.
    fn on_exit(&self, _ctx: &mut StateContext<'_, '_>) -> Result<Vec<StateAction>> {
        Ok(Vec::new())
    }

    /// Handles an event while in this state.
    ///
    /// Returns an EventResult containing the new state (if transition should occur),
    /// actions to execute, and SSE events to emit.
    fn handle_event(&self, event: ActorEvent, ctx: &mut StateContext<'_, '_>) -> Result<EventResult>;
}

/// Context passed to state handlers containing all necessary dependencies.
///
/// This context provides access to database connections, services,
/// and shared state needed by state handlers.
pub struct StateContext<'a, 'b> {
    /// The chat session ID
    pub chat_id: Uuid,

    /// The workspace ID
    pub workspace_id: Uuid,

    /// The user ID who owns this session
    pub user_id: Uuid,

    /// Database connection pool
    pub pool: DbPool,

    /// File storage service
    pub storage: Arc<FileStorageService>,

    /// SSE event broadcaster
    pub event_tx: broadcast::Sender<SseEvent>,

    /// Default persona to use for AI interactions
    pub default_persona: String,

    /// Default context token limit
    pub default_context_token_limit: usize,

    /// Shared actor state (optional - for gradual migration)
    pub shared_state: Option<&'a Arc<tokio::sync::Mutex<SharedActorState>>>,

    /// Optional responder for Pause/Cancel commands
    pub responder: Option<&'b Arc<Mutex<Option<oneshot::Sender<Result<bool>>>>>>,
}

/// Shared state for ChatActor (simplified - no agent cache).
///
/// This state is accessed by all state handlers and should be
/// kept minimal to avoid lock contention.
#[derive(Debug, Default)]
pub struct SharedActorState {
    /// Tool tracking - current tool being executed
    pub current_tool_name: Option<String>,
    pub current_tool_args: Option<serde_json::Value>,

    /// Reasoning tracking (for audit trail)
    pub current_reasoning_id: Option<String>,
    pub reasoning_buffer: Vec<String>,

    /// Interaction state
    pub current_cancellation_token: Option<tokio_util::sync::CancellationToken>,
    pub current_model: Option<String>,
    pub current_task: Option<String>,
    pub is_actively_processing: bool,
}

impl SharedActorState {
    /// Ensures a reasoning ID exists for the current session.
    /// If one doesn't exist, generates a new UUID v7.
    pub fn ensure_reasoning_id(&mut self) -> String {
        self.current_reasoning_id
            .get_or_insert_with(|| uuid::Uuid::now_v7().to_string())
            .clone()
    }
}

/// Registry of state handlers.
///
/// Provides access to the appropriate handler for each state.
pub struct StateHandlerRegistry {
    idle: idle::IdleState,
    running: running::RunningState,
    paused: paused::PausedState,
    error: error::ErrorState,
    cancelled: cancelled::CancelledState,
    completed: completed::CompletedState,
    #[allow(dead_code)]
    shutdown: shutdown::ShutdownState,
}

impl StateHandlerRegistry {
    /// Creates a new state handler registry.
    pub fn new() -> Self {
        Self {
            idle: idle::IdleState::new(),
            running: running::RunningState::new(),
            paused: paused::PausedState::new(),
            error: error::ErrorState::new(),
            cancelled: cancelled::CancelledState::new(),
            completed: completed::CompletedState::new(),
            shutdown: shutdown::ShutdownState::new(),
        }
    }

    /// Gets the handler for the given state.
    pub fn get_handler(&self, state: ActorState) -> &dyn StateHandler {
        match state {
            ActorState::Idle => &self.idle,
            ActorState::Running => &self.running,
            ActorState::Paused => &self.paused,
            ActorState::Error => &self.error,
            ActorState::Cancelled => &self.cancelled,
            ActorState::Completed => &self.completed,
        }
    }
}

impl Default for StateHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_actor_state_default() {
        let state = SharedActorState::default();
        assert!(state.current_tool_name.is_none());
        assert!(state.current_reasoning_id.is_none());
        assert!(state.reasoning_buffer.is_empty());
        assert!(!state.is_actively_processing);
    }

    #[test]
    fn test_state_handler_registry() {
        let registry = StateHandlerRegistry::new();

        // Test that we can get handlers for all states
        let _ = registry.get_handler(ActorState::Idle);
        let _ = registry.get_handler(ActorState::Running);
        let _ = registry.get_handler(ActorState::Paused);
        let _ = registry.get_handler(ActorState::Error);
        let _ = registry.get_handler(ActorState::Cancelled);
        let _ = registry.get_handler(ActorState::Completed);
    }
}
