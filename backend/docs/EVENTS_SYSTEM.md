← [Back to Index](./README.md) | **Related**: [Agent State Machine](./AGENT_STATE_MACHINE.md)

# Events System Specification

## Overview

The Events System provides a **hybrid architecture** for handling ChatActor events by combining **state handlers** and **event processors**. This design separates concerns:

- **State Handlers**: State entry/exit hooks, state validation, state-specific overrides
- **Event Processors**: Event-specific logic, state transitions, actions to execute

## Architecture

### Hybrid Approach

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Event Processing Flow                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  AgentCommand                                                               │
│       │                                                                     │
│       ▼                                                                     │
│  command_to_event() → ActorEvent                                           │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    State Handler Layer                             │    │
│  │                                                                     │    │
│  │  1. Get handler for current_state                                   │    │
│  │  2. Check for state-specific behavior (e.g., "already paused")      │    │
│  │  3. Delegate to Event Processor                                     │    │
│  │  4. Combine results with state hooks                                │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                   Event Processor Layer                             │    │
│  │                                                                     │    │
│  │  1. Get processor for event_type                                   │    │
│  │  2. Execute event-specific logic                                    │    │
│  │  3. Return EventResult with:                                       │    │
│  │     - new_state (transition target)                                │    │
│  │     - actions (UpdateSessionStatus, StartProcessing, etc.)         │    │
│  │     - emit_sse (SSE events to broadcast)                           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│       │                                                                     │
│       ▼                                                                     │
│  execute_state_actions() → Execute actions (after state transition)        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Separation of Concerns

| Component | Responsibility | Location |
|-----------|---------------|----------|
| **State Handlers** | - State entry/exit hooks (`on_enter`, `on_exit`)<br>- State validation<br>- Delegation to event processors<br>- State-specific behavior overrides | `src/services/chat/states/` |
| **Event Processors** | - Event-specific logic<br>- State transitions<br>- Actions to execute<br>- SSE events to emit | `src/services/chat/events/` |

## Event Processors

### Available Processors

| Processor | Event Type | Responsibility |
|-----------|------------|---------------|
| `ProcessInteractionProcessor` | `ProcessInteraction` | Start AI processing, transition to Running |
| `PauseProcessor` | `Pause` | Cancel any ongoing interaction, transition to Paused |
| `CancelProcessor` | `Cancel` | Cancel interaction, transition to Cancelled (terminal) |
| `PingProcessor` | `Ping` | Reset inactivity timer, emit ping SSE event |
| `ShutdownProcessor` | `Shutdown` | Transition to Completed (terminal), trigger actor shutdown |

### Processor Interface

All event processors implement the `EventProcessor` trait:

```rust
pub trait EventProcessor: Send + Sync {
    /// Returns the event type this processor handles
    fn event_type(&self) -> &'static str;

    /// Validates and processes the event, returning the result
    fn execute(&self, event: ActorEvent, ctx: &mut StateContext) -> Result<EventResult>;
}
```

### EventProcessorRegistry

The `EventProcessorRegistry` manages all processors and routes events to the appropriate processor:

```rust
pub struct EventProcessorRegistry {
    processors: HashMap<&'static str, Box<dyn EventProcessor>>,
}

impl EventProcessorRegistry {
    pub fn new(pool: DbPool, storage: Arc<FileStorageService>, ...) -> Self;
    pub fn get_processor(&self, event: &ActorEvent) -> Option<&dyn EventProcessor>;
}
```

## State Handlers

### Handler Interface

All state handlers implement the `StateHandler` trait:

```rust
pub trait StateHandler: Send + Sync {
    fn state(&self) -> ActorState;

    fn on_enter(&self, ctx: &mut StateContext) -> Result<Vec<StateAction>>;

    fn on_exit(&self, ctx: &mut StateContext) -> Result<Vec<StateAction>>;

    fn handle_event(&self, event: ActorEvent, ctx: &mut StateContext) -> Result<EventResult>;
}
```

### StateContext

The `StateContext` provides all dependencies needed by both state handlers and event processors:

```rust
pub struct StateContext<'a, 'b> {
    pub chat_id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub pool: DbPool,
    pub storage: Arc<FileStorageService>,
    pub event_tx: broadcast::Sender<SseEvent>,
    pub default_persona: String,
    pub default_context_token_limit: usize,
    pub shared_state: Option<&'a Arc<Mutex<SharedActorState>>>,
    pub responder: Option<&'b Arc<Mutex<Option<oneshot::Sender<Result<bool>>>>>>,
    pub event_processor_registry: Option<&'a EventProcessorRegistry>,
}

impl<'a, 'b> StateContext<'a, 'b> {
    pub fn get_event_processor(&self, event: &ActorEvent) -> Result<&'a dyn EventProcessor>;
}
```

## Event Processing Examples

### Example 1: ProcessInteraction from Idle

```
User sends message
      │
      ▼
AgentCommand::ProcessInteraction { user_id }
      │
      ▼
IdleState.handle_event(ProcessInteraction, ctx)
      │
      ├─ Check: No state-specific override needed
      ├─ Delegate: ctx.get_event_processor(&event)
      │                 │
      │                 ▼
      │         ProcessInteractionProcessor.execute()
      │                 │
      │                 ├─ Returns: EventResult {
      │                 │     new_state: Some(Running),
      │                 │     actions: [SetActivelyProcessing(true),
      │                 │              UpdateSessionStatus(Running),
      │                 │              StartProcessing { user_id }],
      │                 │     emit_sse: []
      │                 │   }
      │                 │
      ├─ Combine with state hooks (none for this case)
      │
      └─ Return EventResult to ChatActor
            │
            ▼
      execute_state_actions()
            │
            ├─ SetActivelyProcessing(true)
            ├─ UpdateSessionStatus(Running)
            ├─ StartProcessing { user_id } → Spawns background AI task
            │
            ▼
      State transition: Idle → Running
            │
            ▼
      Actor now in Running state, AI processing in background
```

### Example 2: Pause from Running

```
User clicks Pause
      │
      ▼
AgentCommand::Pause { reason: "User paused" }
      │
      ▼
RunningState.handle_event(Pause, ctx)
      │
      ├─ Check: No state-specific override needed
      ├─ Delegate: ctx.get_event_processor(&event)
      │                 │
      │                 ▼
      │         PauseProcessor.execute()
      │                 │
      │                 ├─ Returns: EventResult {
      │                 │     new_state: Some(Paused),
      │                 │     actions: [UpdateSessionStatus(Paused),
      │                 │              CancelInteraction,
      │                 │              SendSuccessResponse],
      │                 │     emit_sse: []
      │                 │   }
      │                 │
      ├─ Combine with state hooks (none for this case)
      │
      └─ Return EventResult to ChatActor
            │
            ▼
      execute_state_actions()
            │
            ├─ UpdateSessionStatus(Paused)
            ├─ CancelInteraction → Signals cancellation token
            ├─ SendSuccessResponse → Sends 200 OK to client
            │
            ▼
      State transition: Running → Paused
            │
            ▼
      Actor now in Paused state, AI task cancelled
```

### Example 3: Pause from Paused (State-Specific Override)

```
User clicks Pause again (already paused)
      │
      ▼
AgentCommand::Pause { reason: "Already paused" }
      │
      ▼
PausedState.handle_event(Pause, ctx)
      │
      ├─ Check: State-specific override! "already paused"
      │         └─ Return custom EventResult {
      │               new_state: None,  // No state change
      │               actions: [SendSuccessResponse],
      │               emit_sse: [Ping]
      │             }
      │
      └─ Skip event processor delegation
            │
            ▼
      execute_state_actions()
            │
            ├─ SendSuccessResponse → Sends 200 OK to client
            │
            ▼
      No state transition (stays in Paused)
```

## Implementation Files

```
backend/src/services/chat/
├── events/
│   ├── mod.rs                    # EventProcessor trait, EventProcessorRegistry
│   ├── process_interaction.rs    # ProcessInteractionProcessor
│   ├── pause.rs                  # PauseProcessor
│   ├── cancel.rs                 # CancelProcessor
│   ├── ping.rs                   # PingProcessor
│   └── shutdown.rs               # ShutdownProcessor
│
└── states/
    ├── mod.rs                    # StateHandler trait, StateContext
    ├── idle.rs                   # IdleState
    ├── running.rs                # RunningState
    ├── paused.rs                 # PausedState
    ├── error.rs                  # ErrorState
    ├── cancelled.rs              # CancelledState
    └── completed.rs              # CompletedState
```

## Key Design Decisions

### 1. Why Hybrid Architecture?

**Problem:** Pure event-centric or pure state-centric approaches have limitations:

- **Event-centric**: Difficult to handle state-specific behavior (e.g., "already paused")
- **State-centric**: Leads to code duplication across states for common events

**Solution:** Hybrid approach with:
- Event processors for shared event logic
- State handlers for state-specific hooks and overrides

### 2. Why Manual Retry?

**Problem:** Automatic retry loops can:
- Consume resources without user intent
- Lead to unexpected costs (API calls)
- Make debugging difficult

**Solution:** Manual retry with:
- Clear error SSE events to client
- Session returns to Idle (ready for retry)
- User decides whether to retry based on error message

### 3. Why Separate Actions from State Transitions?

**Problem:** Actions that execute mid-transition can see inconsistent state.

**Solution:** State transition happens BEFORE actions execute:
1. State changes atomically
2. Actions execute in new state context
3. Database updates reflect final state

## Related Documentation

- [Agent State Machine](./AGENT_STATE_MACHINE.md) - Complete state machine specification
- [Agentic Engine](./AGENTIC_ENGINE.md) - Overall agent architecture
- [REST API Guide](./REST_API_GUIDE.md) - API endpoints for agent control
