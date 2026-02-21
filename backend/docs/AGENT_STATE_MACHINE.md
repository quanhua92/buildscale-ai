← [Back to Index](./README.md) | **Related**: [Agentic Engine](./AGENTIC_ENGINE.md)

# Agent State Machine Specification

## Overview

The ChatActor uses a finite state machine (FSM) to manage agent lifecycle and state transitions. This provides clear, explicit state management with type-safe transitions and comprehensive event handling.

## Architecture

The state machine is implemented in `backend/src/services/chat/state_machine/` with:

- **State Definitions**: `ActorState` enum with all possible states
- **Event Definitions**: `ActorEvent` enum for all state-changing events
- **State Handlers**: Modular handlers for each state's behavior
- **Transition Logic**: Validated state transitions with explicit rules

## States

| State | Description | Terminal |
|-------|-------------|----------|
| `Idle` | Agent is waiting for user input or events | No |
| `Running` | Agent is actively processing an interaction | No |
| `Paused` | Agent is temporarily paused | No |
| `Error` | Agent encountered an error | **Yes** |
| `Cancelled` | Agent was cancelled by user | **Yes** |
| `Completed` | Agent completed naturally (timeout) | **Yes** |

### Terminal States

Once an agent enters a terminal state (`Error`, `Cancelled`, `Completed`):
- **No further state transitions are possible**
- **The actor will shut down automatically**
- **The session is preserved in the database** for audit/history
- **Recreating the actor** requires a new interaction (which spawns a fresh actor)

## State Transitions

```
                    ┌─────────────────────────────────────────┐
                    │                                         │
                    ▼                                         │
┌─────────┐  ProcessInteraction  ┌──────────┐  InteractionComplete  ┌─────────┐
│ Created │ ──────────────────>  │  Idle    │ ──────────────────────> │ Running │
└─────────┘                     └──────────┘                      └─────────┘
                                          │                              │
                                          │ Pause                        │
                                          ▼                              │
                                    ┌──────────┐                 InteractionComplete
                                    │  Paused  │ <─────────────────────────────────┐
                                    └──────────┶────────────────────────────────────┤
                                            │ (resume)                        (success)
                                            ▼                                  │
                                          ┌──────────┐                         │
                                          │  Idle    │ <───────────────────────────┘
                                          └──────────┘
                                            │
                                            │ InactivityTimeout
                                            ▼
                                      ┌────────────┐
                                      │ Completed  │ (terminal)
                                      └────────────┘

                    ┌─────────────┐
                    │   Error     │ (terminal) - from any state on error
                    └─────────────┘

                    ┌─────────────┐
                    │  Cancelled  │ (terminal) - from any state on Cancel
                    └─────────────┘
```

### Transition Rules

| From State | Event | To State | Notes |
|------------|-------|----------|-------|
| Idle | ProcessInteraction | Running | Begin processing user message |
| Idle | Pause | Paused | Pause while idle |
| Idle | Cancel | Cancelled | Terminal - actor shuts down |
| Idle | InactivityTimeout | Completed | Terminal - actor shuts down |
| Running | InteractionComplete (success) | Idle | Return to idle after completion |
| Running | InteractionComplete (error) | Error | Terminal - actor shuts down |
| Running | Pause | Paused | Pause during processing |
| Running | Cancel | Cancelled | Terminal - actor shuts down |
| Paused | ProcessInteraction | Idle | Resume by processing interaction |
| Paused | InactivityTimeout | Completed | Terminal - actor shuts down |
| Any | Error | Error | Terminal - actor shuts down |
| Any | Cancel | Cancelled | Terminal - actor shuts down |

## Events

| Event | Description | Payload |
|-------|-------------|---------|
| `ProcessInteraction` | Start processing a user message | `user_id: Uuid` |
| `Pause` | Pause the current session | `reason: Option<String>` |
| `Cancel` | Cancel the current session | `reason: String` |
| `Ping` | Keep-alive heartbeat | None |
| `Shutdown` | Graceful shutdown request | None |
| `InteractionComplete` | Signal interaction completion | `success: bool, error: Option<String>` |
| `InactivityTimeout` | Fired when idle timeout expires | None |

## State Handlers

Each state has a dedicated handler in `backend/src/services/chat/states/`:

- **`idle.rs`**: Waits for user input or events
- **`running.rs`**: Actively processing user interaction
- **`paused.rs`**: Temporarily suspended state
- **`error.rs`**: Terminal error state
- **`cancelled.rs`**: Terminal cancelled state
- **`completed.rs`**: Terminal completed state

### Handler Interface

All state handlers implement the `StateHandler` trait:

```rust
pub trait StateHandler: Send + Sync {
    fn state(&self) -> ActorState;

    fn on_enter(&self, ctx: &mut StateContext) -> Result<Vec<StateAction>>;

    fn on_exit(&self, ctx: &mut StateContext) -> Result<Vec<StateActions>>;

    fn handle_event(&self, event: ActorEvent, ctx: &mut StateContext) -> Result<EventResult>;
}
```

## SSE Events

The state machine emits SSE events for state changes:

### StateChanged Event

```json
{
  "type": "state_changed",
  "data": {
    "from_state": "idle",
    "to_state": "running",
    "reason": "Processing user interaction"
  }
}
```

### Stopped Event (Terminal)

```json
{
  "type": "stopped",
  "data": {
    "reason": "user_cancelled",
    "partial_response": "Text generated before stop..."
  }
}
```

## Agent Lifecycle

### Spawn

When an actor is spawned:
1. Creates session in database (status: `Idle`)
2. Starts heartbeat task
3. Enters `Idle` state
4. Begins processing commands

### Shutdown

Actor shuts down when:
- **Terminal state reached** (Error, Cancelled, Completed)
- **Inactivity timeout** (10 minutes of no commands)
- **Process crash** (handled byTokio supervision)

Before shutdown:
1. Final state persisted to database
2. Heartbeat task cancelled
3. Command channel closed
4. Event bus remains active (for SSE connections)

### Respawning

When a new interaction arrives for a chat with no active actor:
1. New actor spawned
2. Session status updated (if exists)
3. Actor hydrated from database history
4. Processing begins

## Implementation Files

```
backend/src/services/chat/
├── state_machine/
│   ├── mod.rs              # Public exports
│   ├── state.rs            # ActorState enum
│   ├── event.rs            # ActorEvent enum
│   ├── transition.rs       # Transition validation
│   └── machine.rs          # StateMachine<S,E> implementation
│
├── states/
│   ├── mod.rs              # StateHandler trait
│   ├── idle.rs             # Idle state handler
│   ├── running.rs          # Running state handler
│   ├── paused.rs           # Paused state handler
│   ├── error.rs            # Error state handler (terminal)
│   ├── cancelled.rs        # Cancelled state handler (terminal)
│   └── completed.rs        # Completed state handler (terminal)
│
└── actor.rs                # ChatActor orchestrator
```

## Related Documentation

- [Agentic Engine](./AGENTIC_ENGINE.md) - Overall agent architecture
- [REST API Guide](./REST_API_GUIDE.md) - API endpoints for agent control
- [Agent Swarms](./AGENT_SWARMS.md) - Session management
