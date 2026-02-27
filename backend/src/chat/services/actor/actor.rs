use crate::models::agent_session::SessionStatus;
use crate::models::chat::{ChatMessageRole, NewChatMessage};
use crate::models::sse::SseEvent;
use crate::providers::Agent;
use crate::queries;
use crate::services::agent_sessions;
use crate::services::chat::events;
use crate::services::chat::registry::{AgentCommand, AgentHandle, AgentRegistry};
use crate::chat::services::engine::RigService;
use crate::services::chat::ChatService;
use crate::services::chat::state_machine::{ActorEvent, ActorState, StateMachine, StateAction};
use crate::services::chat::states::{SharedActorState, StateContext, StateHandlerRegistry};
use crate::services::storage::FileStorageService;
use crate::state::{TagIndexMessage, LinkIndexMessage};
use crate::DbPool;
use crate::error::Result;
use rig::streaming::StreamingChat;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, instrument};
use uuid::Uuid;

// Import state machine utilities
use super::state_machine::command_to_event;
// Import interaction processor for background task execution
use super::interaction::{ProcessorContext, process_agent_stream, get_or_create_agent};

// ============================================================================
// NON-BLOCKING TASK TYPES
// ============================================================================

/// Result from a background process_interaction task
enum InteractionResult {
    Success,
    Failed { error: String, is_user_cancellation: bool },
}

/// Handle for a background interaction task
struct BackgroundInteractionTask {
    join_handle: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

/// Context needed to run process_interaction independently
struct InteractionContext {
    chat_id: Uuid,
    workspace_id: Uuid,
    user_id: Uuid,
    pool: DbPool,
    rig_service: Arc<RigService>,
    storage: Arc<FileStorageService>,
    registry: Arc<AgentRegistry>,
    session_id: Option<Uuid>,
    default_persona: String,
    default_context_token_limit: usize,
    state: Arc<Mutex<SharedActorState>>,
    event_tx: broadcast::Sender<SseEvent>,
    /// Channel to signal tag indexer worker when files are modified
    tag_index_tx: mpsc::UnboundedSender<TagIndexMessage>,
    /// Channel to signal link indexer worker when files are modified
    link_index_tx: mpsc::UnboundedSender<LinkIndexMessage>,
}

pub struct ChatActor {
    chat_id: Uuid,
    workspace_id: Uuid,
    user_id: Uuid,
    pool: DbPool,
    rig_service: Arc<RigService>,
    storage: Arc<FileStorageService>,
    registry: Arc<AgentRegistry>,
    command_rx: mpsc::Receiver<AgentCommand>,
    event_tx: broadcast::Sender<SseEvent>,
    default_persona: String,
    default_context_token_limit: usize,
    inactivity_timeout: std::time::Duration,
    /// Agent session ID for tracking this actor in the database
    session_id: Option<Uuid>,
    /// Handle for the heartbeat task
    heartbeat_handle: Option<JoinHandle<()>>,
    /// Unified state for actor and state handlers
    /// Consolidated from ChatActorState and SharedActorState
    state: Arc<Mutex<SharedActorState>>,
    /// State machine for managing actor lifecycle
    state_machine: StateMachine,
    /// State handlers for state-specific behavior
    state_handlers: StateHandlerRegistry,
    /// Event processor registry for event-specific logic
    event_processor_registry: Arc<events::EventProcessorRegistry>,
    /// Channel for receiving background interaction results
    interaction_result_tx: mpsc::Sender<InteractionResult>,
    interaction_result_rx: mpsc::Receiver<InteractionResult>,
    /// Handle for the current background interaction task (if any)
    background_task: Option<BackgroundInteractionTask>,
    /// Channel to signal tag indexer worker when files are modified
    tag_index_tx: mpsc::UnboundedSender<TagIndexMessage>,
    /// Channel to signal link indexer worker when files are modified
    link_index_tx: mpsc::UnboundedSender<LinkIndexMessage>,
}

pub struct ChatActorArgs {
    pub chat_id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub pool: DbPool,
    pub rig_service: Arc<RigService>,
    pub storage: Arc<FileStorageService>,
    pub registry: Arc<AgentRegistry>,
    pub default_persona: String,
    pub default_context_token_limit: usize,
    pub event_tx: broadcast::Sender<SseEvent>,
    pub inactivity_timeout: std::time::Duration,
    /// Channel to signal tag indexer worker when files are modified
    pub tag_index_tx: mpsc::UnboundedSender<TagIndexMessage>,
    /// Channel to signal link indexer worker when files are modified
    pub link_index_tx: mpsc::UnboundedSender<LinkIndexMessage>,
}

impl ChatActor {
    pub fn spawn(
        args: ChatActorArgs,
    ) -> AgentHandle {
        Self::spawn_with_args(args)
    }

    fn spawn_with_args(args: ChatActorArgs) -> AgentHandle {
        let (command_tx, command_rx) = mpsc::channel(32);
        let (interaction_result_tx, interaction_result_rx) = mpsc::channel(1);
        let event_tx = args.event_tx.clone();

        // Initialize state machine in Idle state
        let state_machine = StateMachine::new(ActorState::Idle);

        // Initialize state handlers registry
        let state_handlers = StateHandlerRegistry::new();

        // Initialize event processor registry
        let event_processor_registry = Arc::new(events::EventProcessorRegistry::new(
            args.pool.clone(),
            args.storage.clone(),
            args.event_tx.clone(),
            args.default_persona.clone(),
            args.default_context_token_limit,
        ));

        let actor = Self {
            chat_id: args.chat_id,
            workspace_id: args.workspace_id,
            user_id: args.user_id,
            pool: args.pool.clone(),
            rig_service: args.rig_service,
            storage: args.storage,
            registry: args.registry,
            command_rx,
            event_tx: args.event_tx.clone(),
            default_persona: args.default_persona,
            default_context_token_limit: args.default_context_token_limit,
            inactivity_timeout: args.inactivity_timeout,
            session_id: None,
            heartbeat_handle: None,
            state: Arc::new(Mutex::new(SharedActorState::default())),
            state_machine,
            state_handlers,
            event_processor_registry,
            interaction_result_tx,
            interaction_result_rx,
            background_task: None,
            tag_index_tx: args.tag_index_tx,
            link_index_tx: args.link_index_tx,
        };

        tokio::spawn(async move {
            actor.run().await;
        });

        AgentHandle {
            command_tx,
            event_tx,
        }
    }

    async fn run(mut self) {
        // ===========================================================================
        // MAIN EVENT LOOP
        // ===========================================================================
        tracing::info!(
            chat_id = %self.chat_id,
            workspace_id = %self.workspace_id,
            user_id = %self.user_id,
            inactivity_timeout_secs = self.inactivity_timeout.as_secs(),
            "[ChatActor] STARTED - Agent lifecycle beginning"
        );

        // Log when entering the main loop
        tracing::debug!(
            chat_id = %self.chat_id,
            "[ChatActor] Entering main event loop"
        );

        // Create agent session in database
        let session_result = self.create_session().await;

        // Treat session creation failure as fatal error - shut down actor
        let session_id = match session_result {
            Ok(id) => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    session_id = %id,
                    "[ChatActor] Session created, starting heartbeat task"
                );
                id
            }
            Err(e) => {
                tracing::error!(
                    chat_id = %self.chat_id,
                    error = ?e,
                    "[ChatActor] Failed to create session - shutting down actor"
                );
                return;
            }
        };

        self.session_id = Some(session_id);
        self.heartbeat_handle = Some(self.start_heartbeat_task(session_id));

        // Log initial state entry (actor starts in Idle state)
        self.log_state_transition(
            ActorState::Idle,
            ActorState::Idle,
            "Actor started - entering Idle state"
        );

        // Periodic heartbeat ping (every 10 seconds)
        let mut heartbeat_interval = tokio::time::interval(std::time::Duration::from_secs(10));

        // Inactivity timeout (shutdown after no commands)
        let inactivity_timeout_duration = self.inactivity_timeout;
        let inactivity_timeout = tokio::time::sleep(inactivity_timeout_duration);
        tokio::pin!(inactivity_timeout);

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    let _ = self.event_tx.send(SseEvent::Ping);
                }
                _ = &mut inactivity_timeout => {
                    // Only timeout if NOT actively processing
                    let is_actively_processing = self.state.lock().await.is_actively_processing;

                    if !is_actively_processing {
                        tracing::info!(
                            chat_id = %self.chat_id,
                            reason = "inactivity_timeout",
                            "[ChatActor] SHUTTING DOWN - No commands received while idle"
                        );

                        // State transition: → Completed (terminal state)
                        let _ = self.transition_state(
                            ActorEvent::InactivityTimeout,
                            "Inactivity timeout reached"
                        ).await;

                        // Update session status to completed in database
                        if let Some(session_id) = self.session_id {
                            let _ = self.update_session_status(
                                session_id,
                                crate::models::agent_session::SessionStatus::Completed,
                                None,
                            ).await;
                            tracing::debug!(
                                chat_id = %self.chat_id,
                                session_id = %session_id,
                                "[ChatActor] Session marked as completed due to inactivity timeout"
                            );
                        }

                        break;
                    }

                    // Reset timeout if actively processing - the actor is busy
                    tracing::debug!(
                        chat_id = %self.chat_id,
                        "[ChatActor] Inactivity timeout fired but actor is actively processing, resetting timeout"
                    );
                    inactivity_timeout.as_mut().reset(tokio::time::Instant::now() + inactivity_timeout_duration);
                }
                command = self.command_rx.recv() => {
                    if let Some(cmd) = command {
                        // Reset inactivity timeout on any command
                        inactivity_timeout.as_mut().reset(tokio::time::Instant::now() + inactivity_timeout_duration);

                        // Try to process the command through state handlers first
                        // If the handler returns true, the command was fully handled
                        // If the handler returns false or errors, fall back to legacy path
                        let handled_by_state_handler = match self.process_command_via_state_handler(&cmd).await {
                            Ok(handled) => handled,
                            Err(e) => {
                                warn!(
                                    error = ?e,
                                    "[ChatActor] State handler processing failed, falling back to legacy path"
                                );
                                false
                            }
                        };

                        // If state handler handled it, continue to next command
                        if handled_by_state_handler {
                            // Check if we should break (terminal state)
                            if self.current_state().is_terminal() {
                                break;
                            }
                            continue;
                        }

                        // Note: All AgentCommand variants are now handled by the state machine via
                        // command_to_event() and process_command_via_state_handler(). The legacy fallback
                        // has been removed since process_command_via_state_handler will always handle
                        // the command (handled_by_state_handler will be true).
                    } else {
                        break;
                    }
                }
                // Receive background interaction results
                Some(result) = self.interaction_result_rx.recv() => {
                    self.background_task = None;

                    // Clear current_task from session
                    if let Some(session_id) = self.session_id {
                        if let Ok(mut conn) = self.pool.acquire().await {
                            let _ = agent_sessions::update_session_task(&mut conn, session_id, None, self.user_id).await;
                        }
                    }

                    // Create the InteractionComplete event based on result
                    let (success, error_msg) = match result {
                        InteractionResult::Success => (true, None),
                        InteractionResult::Failed { error, is_user_cancellation } => {
                            // Send error SSE event for non-cancellation failures
                            if !is_user_cancellation {
                                let _ = self.event_tx.send(SseEvent::Error { message: format!("AI Engine Error: {}", error) });
                            }
                            (false, Some(error))
                        }
                    };

                    let event = ActorEvent::InteractionComplete { success, error: error_msg.clone() };

                    // Clone event for use after handler.move() (for both transition and logging)
                    let event_for_transition = event.clone();
                    let event_for_log = event.clone();

                    // Process through state handler (same pattern as commands)
                    let current_state = self.current_state();
                    let handler = self.state_handlers.get_handler(current_state);
                    let mut ctx = self.create_state_context();

                    match handler.handle_event(event, &mut ctx) {
                        Ok(event_result) => {
                            // Separate response actions from other actions
                            let mut other_actions = Vec::new();
                            for action in event_result.actions {
                                if matches!(
                                    action,
                                    StateAction::SendSuccessResponse | StateAction::SendFailureResponse { .. }
                                ) {
                                    // Response actions not applicable for background results
                                } else {
                                    other_actions.push(action);
                                }
                            }

                            // State transition MUST happen BEFORE executing actions
                            if let Some(new_state) = event_result.new_state {
                                let _ = self.transition_state(
                                    event_for_transition,
                                    &format!("Background interaction: {:?} → {:?}", current_state, new_state)
                                ).await;
                            }

                            // Execute actions AFTER state transition (includes UpdateSessionStatus)
                            let _ = self.execute_state_actions(other_actions).await;

                            // Emit any SSE events
                            for sse_event in event_result.emit_sse {
                                let _ = self.event_tx.send(sse_event);
                            }

                            // Check if we should break (terminal state)
                            if event_result.new_state.map_or(false, |s| s.is_terminal()) {
                                break;
                            }
                        }
                        Err(e) => {
                            warn!(
                                current_state = %current_state,
                                event = ?event_for_log,
                                error = ?e,
                                "[ChatActor] State handler failed for InteractionComplete, attempting to update session status"
                            );
                            // Fallback: try to at least update the session status to Idle
                            if let Some(session_id) = self.session_id {
                                let new_status = if success {
                                    SessionStatus::Idle
                                } else {
                                    SessionStatus::Idle // Transient errors return to Idle for retry
                                };
                                let _ = self.update_session_status(session_id, new_status, error_msg).await;
                            }
                        }
                    }

                    self.state.lock().await.is_actively_processing = false;
                }
            }
        }

        // Cleanup: stop heartbeat and mark session as completed
        let heartbeat_handle = self.heartbeat_handle.take();
        let session_id = self.session_id.take();

        // Cancel any background task
        if let Some(task) = self.background_task.take() {
            task.cancellation_token.cancel();
            task.join_handle.abort();
        }

        tracing::info!(
            chat_id = %self.chat_id,
            session_id = ?session_id,
            "[ChatActor] ACTOR EXITING - cleanup starting"
        );

        if let Some(handle) = heartbeat_handle {
            handle.abort();
        }

        // Note: We intentionally do NOT mark the session as completed here.
        // The session record should persist in its last known state (e.g., idle, running).
        // - User-cancelled sessions: handler/service owns final DB state
        // - Inactivity timeout: session is NOT "completed", just inactive
        // - Normal shutdown: session stays in its last active state
        // Stale sessions will be handled by the cleanup worker if not re-activated.
        tracing::info!(
            chat_id = %self.chat_id,
            session_id = ?session_id,
            "[ChatActor] EXITED - Actor lifecycle complete"
        );
    }

    /// Captures all state needed to run process_interaction in a background task
    fn capture_interaction_context(&self) -> InteractionContext {
        InteractionContext {
            chat_id: self.chat_id,
            workspace_id: self.workspace_id,
            user_id: self.user_id,
            pool: self.pool.clone(),
            rig_service: self.rig_service.clone(),
            storage: self.storage.clone(),
            registry: self.registry.clone(),
            session_id: self.session_id,
            default_persona: self.default_persona.clone(),
            default_context_token_limit: self.default_context_token_limit,
            state: self.state.clone(),
            event_tx: self.event_tx.clone(),
            tag_index_tx: self.tag_index_tx.clone(),
            link_index_tx: self.link_index_tx.clone(),
        }
    }

    // ========================================================================
    // SESSION TRACKING METHODS
    // ========================================================================

    /// Creates a new agent session in the database.
    async fn create_session(&self) -> Result<Uuid> {
        super::session::create_session(
            &self.pool,
            &self.storage,
            self.workspace_id,
            self.chat_id,
            self.user_id,
            &self.default_persona,
        ).await
    }

    /// Updates the session status in the database.
    async fn update_session_status(
        &self,
        session_id: Uuid,
        status: crate::models::agent_session::SessionStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        super::session::update_session_status(
            &self.pool,
            self.chat_id,
            session_id,
            status,
            error_message,
        ).await
    }

    /// Starts a background task that sends periodic heartbeats to the database.
    /// This keeps the session alive and indicates the agent is actively running.
    fn start_heartbeat_task(&self, session_id: Uuid) -> JoinHandle<()> {
        super::session::start_heartbeat_task(
            self.chat_id,
            self.pool.clone(),
            session_id,
        )
    }

    // ========================================================================
    // STATE MACHINE INTEGRATION METHODS
    // ========================================================================

    /// Log a state transition for debugging and monitoring
    #[instrument(skip(self), fields(chat_id = %self.chat_id))]
    fn log_state_transition(&self, from: ActorState, to: ActorState, reason: &str) {
        info!(
            from_state = %from,
            to_state = %to,
            reason = %reason,
            "[ChatActor] State transition"
        );

        // Emit StateChanged SSE event
        let _ = self.event_tx.send(SseEvent::StateChanged {
            from_state: from.to_string(),
            to_state: to.to_string(),
            reason: Some(reason.to_string()),
        });
    }

    /// Get the current state from the state machine
    fn current_state(&self) -> ActorState {
        self.state_machine.current_state()
    }

    /// Transition state using the state machine and log the transition
    #[instrument(skip(self, event, reason), fields(chat_id = %self.chat_id))]
    async fn transition_state(
        &mut self,
        event: ActorEvent,
        reason: &str,
    ) -> Result<()> {
        let from_state = self.current_state();

        // Try to perform the transition
        match self.state_machine.handle_event(event) {
            Ok(transition) if transition.state_changed => {
                self.log_state_transition(from_state, transition.new_state, reason);
                Ok(())
            }
            Ok(_) => {
                // No state change, but not an error
                Ok(())
            }
            Err(e) => {
                warn!(
                    from_state = %from_state,
                    error = ?e,
                    "[ChatActor] State transition failed, continuing with current state"
                );
                // Continue despite failed transition - don't break the actor
                Ok(())
            }
        }
    }

    // ===========================================================================
    // STATE ACTION EXECUTION METHODS
    // ===========================================================================

    /// Execute a state action returned by a state handler.
    #[instrument(skip(self), fields(chat_id = %self.chat_id))]
    async fn execute_state_action(&mut self, action: StateAction) -> Result<()> {
        match action {
            StateAction::UpdateSessionStatus(status) => {
                if let Some(session_id) = self.session_id {
                    tracing::debug!(
                        chat_id = %self.chat_id,
                        session_id = %session_id,
                        new_status = %status,
                        "[ChatActor] Executing UpdateSessionStatus action"
                    );
                    let _ = self.update_session_status(session_id, status, None).await;
                }
            }
            StateAction::SetActivelyProcessing(value) => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    is_actively_processing = value,
                    "[ChatActor] Executing SetActivelyProcessing action"
                );
                let mut state = self.state.lock().await;
                state.is_actively_processing = value;
            }
            StateAction::EmitSse(event) => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    event_type = ?event,
                    "[ChatActor] Executing EmitSse action"
                );
                let _ = self.event_tx.send(event);
            }
            StateAction::ResetInactivityTimer => {
                // This is handled by the main loop's select! statement
                tracing::trace!(
                    chat_id = %self.chat_id,
                    "[ChatActor] ResetInactivityTimer action requested"
                );
            }
            StateAction::ShutdownActor => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Executing ShutdownActor action"
                );
                // This will be handled by breaking out of the main loop
                // The state machine will transition to a terminal state first
            }
            StateAction::SaveResponse(response) => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Executing SaveResponse action"
                );
                // This would save the response to the database
                // For now, this is a placeholder for future implementation
                let _ = response;
            }
            StateAction::CancelInteraction => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Executing CancelInteraction action"
                );
                // Cancel any background task
                if let Some(task) = &self.background_task {
                    task.cancellation_token.cancel();
                }
                // Cancel the current interaction token
                let token = self.state.lock().await.current_cancellation_token.clone();
                if let Some(token) = token {
                    tracing::debug!(
                        chat_id = %self.chat_id,
                        "[ChatActor] Cancelled current interaction token"
                    );
                    token.cancel();
                }
            }
            StateAction::SendSuccessResponse => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Executing SendSuccessResponse action"
                );
                // The responder is handled separately in the command processing
                // This action is just a signal that the command succeeded
            }
            StateAction::SendFailureResponse { message } => {
                tracing::debug!(
                    chat_id = %self.chat_id,
                    message = %message,
                    "[ChatActor] Executing SendFailureResponse action"
                );
                // The responder is handled separately in the command processing
                // This action is just a signal that the command failed
                let _ = message;
            }
            StateAction::StartProcessing { user_id } => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    user_id = %user_id,
                    "[ChatActor] Executing StartProcessing action - spawning background AI interaction task"
                );

                // Guard: Don't start if already running
                if self.background_task.is_some() {
                    tracing::warn!("StartProcessing called while task running - ignoring");
                    return Ok(());
                }

                let cancellation_token = CancellationToken::new();
                self.state.lock().await.current_cancellation_token = Some(cancellation_token.clone());

                let ctx = self.capture_interaction_context();
                let result_tx = self.interaction_result_tx.clone();
                let cancellation_token_for_task = cancellation_token.clone();

                let join_handle = tokio::spawn(async move {
                    let result = process_interaction_standalone(ctx, user_id, cancellation_token_for_task).await;
                    let _ = result_tx.send(result).await;
                });

                self.background_task = Some(BackgroundInteractionTask {
                    join_handle,
                    cancellation_token,
                });
            }
        }
        Ok(())
    }

    /// Execute multiple state actions in sequence.
    async fn execute_state_actions(&mut self, actions: Vec<StateAction>) -> Result<()> {
        for action in actions {
            self.execute_state_action(action).await?;
        }
        Ok(())
    }

    /// Create a state context for state handlers.
    fn create_state_context(&self) -> StateContext<'_, '_> {
        StateContext {
            chat_id: self.chat_id,
            workspace_id: self.workspace_id,
            user_id: self.user_id,
            pool: self.pool.clone(),
            storage: self.storage.clone(),
            event_tx: self.event_tx.clone(),
            default_persona: self.default_persona.clone(),
            default_context_token_limit: self.default_context_token_limit,
            shared_state: Some(&self.state),
            responder: None,
            event_processor_registry: Some(&self.event_processor_registry),
        }
    }

    /// Create a state context with a responder for Pause/Cancel commands.
    fn create_state_context_with_responder<'b>(
        &self,
        responder: &'b Arc<Mutex<Option<oneshot::Sender<crate::error::Result<bool>>>>>,
    ) -> StateContext<'_, 'b> {
        StateContext {
            chat_id: self.chat_id,
            workspace_id: self.workspace_id,
            user_id: self.user_id,
            pool: self.pool.clone(),
            storage: self.storage.clone(),
            event_tx: self.event_tx.clone(),
            default_persona: self.default_persona.clone(),
            default_context_token_limit: self.default_context_token_limit,
            shared_state: Some(&self.state),
            responder: Some(responder),
            event_processor_registry: Some(&self.event_processor_registry),
        }
    }

    /// Process a command through the state handler system.
    ///
    /// Returns true if the command was fully handled by the state system,
    /// false if it needs to be processed by the legacy code path.
    #[instrument(skip(self, command), fields(chat_id = %self.chat_id))]
    async fn process_command_via_state_handler(
        &mut self,
        command: &AgentCommand,
    ) -> Result<bool> {
        // Extract responder from Pause/Cancel commands
        let responder = match command {
            AgentCommand::Pause { responder, .. } => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Pause: responder extracted"
                );
                Some(responder)
            }
            AgentCommand::Cancel { responder, reason: _ } => {
                tracing::info!(
                    chat_id = %self.chat_id,
                    "[ChatActor] Cancel: responder extracted"
                );
                Some(responder)
            }
            _ => None,
        };

        // Convert command to event
        let event = match command_to_event(command) {
            Some(e) => e,
            None => {
                // Command doesn't map to an event, use legacy path
                return Ok(false);
            }
        };

        let current_state = self.current_state();

        // Clone event for use after handle_event (which moves it)
        let event_for_log = event.clone();
        let event_for_transition = event.clone();

        // Get handler before creating mutable context
        let handler = self.state_handlers.get_handler(current_state);

        // Create context with or without responder
        let mut ctx = if let Some(ref responder_arc) = responder {
            self.create_state_context_with_responder(responder_arc)
        } else {
            self.create_state_context()
        };

        // Call the state handler
        let result = handler.handle_event(event, &mut ctx);

        match result {
            Ok(event_result) => {
                tracing::debug!(
                    new_state = ?event_result.new_state,
                    actions_count = event_result.actions.len(),
                    "[ChatActor] State handler result"
                );
                // Remember if there were actions before moving
                let has_actions = !event_result.actions.is_empty();
                let has_state_change = event_result.new_state.is_some();

                // Separate response actions from other actions
                let mut response_actions = Vec::new();
                let mut other_actions = Vec::new();
                for action in event_result.actions {
                    if matches!(
                        action,
                        StateAction::SendSuccessResponse | StateAction::SendFailureResponse { .. }
                    ) {
                        response_actions.push(action);
                    } else {
                        other_actions.push(action);
                    }
                }

                // IMPORTANT: State transition MUST happen BEFORE executing actions
                // This ensures that actions like StartProcessing can send events
                // (like InteractionComplete) that will be handled by the NEW state
                if let Some(new_state) = event_result.new_state {
                    tracing::info!(
                        from = %current_state,
                        to = %new_state,
                        "[ChatActor] Transitioning state BEFORE executing actions"
                    );
                    let _ = self.transition_state(
                        event_for_transition,
                        &format!("State handler: {:?} → {:?}", current_state, new_state)
                    ).await;
                }

                // Execute non-response actions AFTER state transition
                self.execute_state_actions(other_actions).await?;

                // Handle response actions by sending to the responder
                // This MUST happen BEFORE checking for terminal state, so responses are sent
                if let Some(responder_arc) = responder {
                    tracing::info!(
                        chat_id = %self.chat_id,
                        response_actions_count = response_actions.len(),
                        "[ChatActor] Processing response actions"
                    );
                    if let Some(sender_option) = responder_arc.lock().await.take() {
                        tracing::info!(
                            chat_id = %self.chat_id,
                            "[ChatActor] Sending response"
                        );
                        let send_result = if response_actions.iter().any(|a| matches!(a, StateAction::SendSuccessResponse)) {
                            // Success - send Ok(true)
                            sender_option.send(Ok(true))
                        } else if let Some(fail_action) = response_actions.iter().find(|a| matches!(a, StateAction::SendFailureResponse { .. })) {
                            // Failure - send error message
                            if let StateAction::SendFailureResponse { message } = fail_action {
                                sender_option.send(Err(crate::error::Error::Internal(message.clone())))
                            } else {
                                unreachable!()
                            }
                        } else {
                            // No explicit response action, but command was processed successfully
                            sender_option.send(Ok(true))
                        };

                        match &send_result {
                            Ok(_) => tracing::info!(chat_id = %self.chat_id, "[ChatActor] Response sent successfully"),
                            Err(_) => tracing::warn!(chat_id = %self.chat_id, "[ChatActor] Failed to send response"),
                        }
                    } else {
                        tracing::warn!(
                            chat_id = %self.chat_id,
                            "[ChatActor] Responder sender was None (already taken?)"
                        );
                    }
                }

                // NOW check if we should break out of the loop (terminal state)
                // Response has been sent, so we can safely exit
                if let Some(new_state) = event_result.new_state {
                    if new_state.is_terminal() {
                        return Ok(true);
                    }
                }

                // Emit any SSE events
                for sse_event in event_result.emit_sse {
                    let _ = self.event_tx.send(sse_event);
                }

                // Return true if the handler actually did something (state change or actions)
                // Return false to fall through to legacy path
                let handled = has_state_change || has_actions;
                tracing::debug!(
                    handled,
                    has_state_change,
                    has_actions,
                    "[ChatActor] State handler 'handled' result (false=fall through to legacy)"
                );
                Ok(handled)
            }
            Err(e) => {
                // Send failure response to responder if present
                if let Some(responder_arc) = responder {
                    if let Some(sender_option) = responder_arc.lock().await.take() {
                        let error_msg = format!("State handler failed: {}", e);
                        let _ = sender_option.send(Err(crate::error::Error::Internal(error_msg)));
                    }
                }

                warn!(
                    current_state = %current_state,
                    event = ?event_for_log,
                    error = ?e,
                    "[ChatActor] State handler failed, falling back to legacy path"
                );
                Ok(false)
            }
        }
    }
}

// ============================================================================
// STANDALONE FUNCTION FOR BACKGROUND TASK EXECUTION
// ============================================================================

/// Standalone version that can run in a background task
/// This is a wrapper that uses the existing ChatActor method but captures its result
async fn process_interaction_standalone(
    ctx: InteractionContext,
    user_id: Uuid,
    cancellation_token: CancellationToken,
) -> InteractionResult {
    use super::constants::{MAX_AI_RETRIES, RETRY_BACKOFF_MS};
    use super::stream::flush_reasoning_buffer;

    tracing::info!(
        chat_id = %ctx.chat_id,
        user_id = %user_id,
        "[ChatActor] [Background] ProcessInteraction STARTED"
    );

    // Log SSE receiver count
    tracing::debug!(
        chat_id = %ctx.chat_id,
        receivers = ctx.event_tx.receiver_count(),
        "[ChatActor] [Background] Current SSE receiver count"
    );

    // Create a new cancellation token for this interaction
    ctx.state.lock().await.current_cancellation_token = Some(cancellation_token.clone());

    let mut conn = match ctx.pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            return InteractionResult::Failed {
                error: format!("Failed to acquire database connection: {}", e),
                is_user_cancellation: false,
            };
        }
    };

    // 1. Build structured context with persona, history, and attachments
    let context = match ChatService::build_context(
        &mut conn,
        &ctx.storage,
        ctx.workspace_id,
        ctx.chat_id,
        &ctx.default_persona,
        ctx.default_context_token_limit,
        true, // exclude_last_message for AI context
    ).await {
        Ok(ctx_data) => ctx_data,
        Err(e) => {
            return InteractionResult::Failed {
                error: format!("Failed to build context: {}", e),
                is_user_cancellation: false,
            };
        }
    };

    // 2. Get current message (the prompt)
    let messages = match queries::chat::get_messages_by_file_id(&mut conn, ctx.workspace_id, ctx.chat_id).await {
        Ok(msgs) => msgs,
        Err(e) => {
            return InteractionResult::Failed {
                error: format!("Failed to get messages: {}", e),
                is_user_cancellation: false,
            };
        }
    };

    let last_message = match messages.last() {
        Some(msg) => msg,
        None => {
            return InteractionResult::Failed {
                error: "No messages found".to_string(),
                is_user_cancellation: false,
            };
        }
    };

    // Set current task for session tracking
    let task_preview = crate::utils::safe_preview(&last_message.content, 100);
    ctx.state.lock().await.current_task = Some(task_preview.clone());

    // Update session with current task
    if let Some(session_id) = ctx.session_id {
        if let Err(e) = agent_sessions::update_session_task(&mut conn, session_id, Some(task_preview.clone()), ctx.user_id).await {
            tracing::warn!(
                session_id = %session_id,
                error = %e,
                "[ChatActor] [Background] Failed to update session task"
            );
        }
    }

    // 3. Convert history to Rig format
    let history = ctx.rig_service.convert_history_with_attachments(&context.history.messages, Some(&context.attachment_manager));

    // 4. Build prompt
    let prompt = last_message.content.clone();

    // 5. Get agent config from file
    let agent_config = match crate::services::chat::sync::get_agent_config_from_file(
        &mut conn,
        &ctx.storage,
        ctx.workspace_id,
        ctx.chat_id,
    ).await {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!(
                chat_id = %ctx.chat_id,
                error = %e,
                "Failed to get agent config from file, using defaults"
            );
            crate::models::chat::AgentConfig {
                persona_override: Some(context.persona.clone()),
                ..Default::default()
            }
        }
    };

    let session = crate::models::chat::ChatSession {
        file_id: ctx.chat_id,
        agent_config,
        messages: messages.clone(),
    };

    // Store current model for potential cancellation
    ctx.state.lock().await.current_model = Some(session.agent_config.model.clone());

    // 6. Load AI config
    let ai_config = match crate::config::Config::load() {
        Ok(config) => config.ai,
        Err(e) => {
            return InteractionResult::Failed {
                error: format!("Failed to load AI config: {}", e),
                is_user_cancellation: false,
            };
        }
    };

    // 7. Get or create agent
    let processor_ctx = ProcessorContext {
        chat_id: ctx.chat_id,
        workspace_id: ctx.workspace_id,
        user_id: ctx.user_id,
        pool: ctx.pool.clone(),
        rig_service: ctx.rig_service.clone(),
        storage: ctx.storage.clone(),
        registry: ctx.registry.clone(),
        session_id: ctx.session_id,
        default_persona: ctx.default_persona.clone(),
        default_context_token_limit: ctx.default_context_token_limit,
        state: ctx.state.clone(),
        event_tx: ctx.event_tx.clone(),
        tag_index_tx: ctx.tag_index_tx.clone(),
        link_index_tx: ctx.link_index_tx.clone(),
    };

    let agent = match get_or_create_agent(&processor_ctx, user_id, &session, &ai_config).await {
        Ok(a) => a,
        Err(e) => {
            return InteractionResult::Failed {
                error: format!("Failed to get or create agent: {}", e),
                is_user_cancellation: false,
            };
        }
    };

    // 8. Register cancellation token
    ctx.registry.register_cancellation(ctx.chat_id, cancellation_token.clone()).await;

    let mut item_count = 0usize;

    // 9. Process stream with retry logic
    let mut retry_count = 0u32;
    let full_response = loop {
        if cancellation_token.is_cancelled() {
            return InteractionResult::Failed {
                error: "Chat cancelled by user".to_string(),
                is_user_cancellation: true,
            };
        }

        let result = match &agent {
            Agent::OpenAI(openai_agent) => {
                let stream = openai_agent.stream_chat(&prompt, history.clone()).await;
                process_agent_stream(&processor_ctx, stream, &cancellation_token, &mut conn, &session, &mut item_count).await
            }
            Agent::OpenRouter(openrouter_agent) => {
                let stream = openrouter_agent.stream_chat(&prompt, history.clone()).await;
                process_agent_stream(&processor_ctx, stream, &cancellation_token, &mut conn, &session, &mut item_count).await
            }
        };

        match result {
            Ok(response) => break response,
            Err(e) => {
                let error_str = format!("{:?}", e);
                let is_retryable = error_str.contains("Failed to get tool definitions")
                    || error_str.contains("RequestError")
                    || error_str.contains("rate limit")
                    || error_str.contains("timeout")
                    || error_str.contains("connection")
                    || error_str.contains("5")
                    || error_str.contains("overloaded");

                if is_retryable && retry_count < MAX_AI_RETRIES {
                    retry_count += 1;
                    let backoff_ms = RETRY_BACKOFF_MS * (1 << (retry_count - 1));
                    tracing::warn!(
                        chat_id = %ctx.chat_id,
                        retry = retry_count,
                        max_retries = MAX_AI_RETRIES,
                        backoff_ms = backoff_ms,
                        error = ?e,
                        "[ChatActor] [Background] Transient AI error, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                } else {
                    return InteractionResult::Failed {
                        error: format!("AI error: {}", e),
                        is_user_cancellation: false,
                    };
                }
            }
        }
    };

    // 10. Remove cancellation token
    ctx.registry.remove_cancellation(&ctx.chat_id).await;

    // 11. Save Assistant Response
    if !full_response.is_empty() {
        let mut final_conn = match ctx.pool.acquire().await {
            Ok(c) => c,
            Err(e) => {
                return InteractionResult::Failed {
                    error: format!("Failed to acquire connection for saving: {}", e),
                    is_user_cancellation: false,
                };
            }
        };

        if let Err(e) = flush_reasoning_buffer(&ctx.state, ctx.chat_id, ctx.workspace_id, &ctx.storage, &mut final_conn).await {
            tracing::error!(
                chat_id = %ctx.chat_id,
                error = %e,
                "[ChatActor] [Background] Failed to flush reasoning buffer"
            );
        }

        let reasoning_id = ctx.state.lock().await.current_reasoning_id.clone();

        if let Err(e) = ChatService::save_message(
            &mut final_conn,
            &ctx.storage,
            ctx.workspace_id,
            NewChatMessage {
                file_id: ctx.chat_id,
                workspace_id: ctx.workspace_id,
                role: ChatMessageRole::Assistant,
                content: full_response.clone(),
                metadata: sqlx::types::Json(crate::models::chat::ChatMessageMetadata {
                    model: Some(session.agent_config.model.clone()),
                    reasoning_id,
                    ..Default::default()
                }),
            },
        ).await {
            return InteractionResult::Failed {
                error: format!("Failed to save message: {}", e),
                is_user_cancellation: false,
            };
        }
    }

    // Send Done event
    let _ = ctx.event_tx.send(SseEvent::Done {
        message: "Turn complete".to_string(),
    });

    tracing::info!(
        chat_id = %ctx.chat_id,
        "[ChatActor] [Background] ProcessInteraction COMPLETED"
    );

    // Cleanup
    ctx.state.lock().await.current_cancellation_token = None;
    ctx.state.lock().await.current_reasoning_id = None;
    let _current_task = ctx.state.lock().await.current_task.take();

    InteractionResult::Success
}
