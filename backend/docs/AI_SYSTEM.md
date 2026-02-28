# AI System Architecture

BuildScale AI implements a stateful **Agentic Engine** where AI agents live, plan, and execute directly within a workspace environment. This replaces the traditional "AI-as-a-Proxy" model with a structured context system and state machine-driven lifecycle.

## Table of Contents

- [Core Philosophy](#core-philosophy)
- [Agent State Machine](#agent-state-machine)
- [Agent Sessions](#agent-sessions)
- [Context Engineering](#context-engineering)
- [Context Caching](#context-caching)
- [AI Providers](#ai-providers)
- [Rig Integration](#rig-integration)
- [SSE Event Protocol](#sse-event-protocol)

---

## Core Philosophy

### The Agentic Engine

BuildScale is built on the principle that **The Workspace is the OS**:

- **Agentic Engine Authority**: The Rust backend owns the "hands" (Tools API) and the "memory" (File Registry). The LLM is a swappable "processor".
- **Identity vs. Content**: Every object has a permanent Identity (id, path) and immutable Content history.
- **The Write-Flush Loop**: Every AI action is committed to PostgreSQL and flushed to disk before the next "thought".

### Agent Personas

| Persona | Purpose | Mode | Use Case |
|---------|---------|------|----------|
| **Assistant** | General-purpose conversational AI | `chat` | Q&A, debugging, exploration |
| **Planner** | Strategic planning and task breakdown | `plan` | Implementation plans, task analysis |
| **Builder** | Code generation and file manipulation | `build` | Writing code, implementing features |

---

## Agent State Machine

The ChatActor uses a finite state machine (FSM) for clear, explicit state management with type-safe transitions.

### States

| State | Description | Terminal |
|-------|-------------|----------|
| `Idle` | Agent is waiting for user input or events | No |
| `Running` | Agent is actively processing an interaction | No |
| `Paused` | Agent is temporarily paused | No |
| `Error` | Agent encountered an unrecoverable error | **Yes** |
| `Cancelled` | Agent was cancelled by user | **Yes** |
| `Completed` | Agent completed naturally (timeout) | **Yes** |

### Terminal States

Once an agent enters a terminal state (`Error`, `Cancelled`, `Completed`):
- **No further state transitions are possible**
- **The actor will shut down automatically**
- **The session is preserved in the database** for audit/history
- **Recreating the actor** requires a new interaction (spawns fresh actor)

### State Transitions

```
┌─────────┐  ProcessInteraction  ┌──────────┐  ProcessInteraction  ┌─────────┐
│ Created │ ──────────────────>  │  Idle    │ ──────────────────>  │ Running │
└─────────┘                     └──────────┘                      └─────────┘
                                      │                                │
                                      │ Pause                          │
                                      ▼                                │
                                ┌──────────┐                   InteractionComplete
                                │  Paused  │ <───────────────────────────────┐
                                └──────────┘                                 │
                                      │                                      │
                                      │ InactivityTimeout                    │
                                      ▼                                      ▼
                                ┌────────────┐                      ┌─────────────┐
                                │ Completed  │ (terminal)           │  Cancelled  │ (terminal)
                                └────────────┘                      └─────────────┘
```

### Transition Rules

| From State | Event | To State | Notes |
|------------|-------|----------|-------|
| Idle | ProcessInteraction | Running | Begin processing |
| Idle | Pause | Paused | Pause while idle |
| Idle | Cancel | Cancelled | Terminal |
| Idle | InactivityTimeout | Completed | Terminal |
| Running | InteractionComplete | Idle | Return to idle |
| Running | Pause | Paused | Pause during processing |
| Running | Cancel | Cancelled | Terminal |
| Paused | ProcessInteraction | Idle | Resume |
| Any | Error | Error | Terminal |

### Error Handling

| Error Type | Examples | State Transition | Retry |
|------------|----------|------------------|-------|
| **Transient** | API timeout, rate limit | Running → **Idle** | Manual retry allowed |
| **Unrecoverable** | Corrupt data, critical failure | Any → **Error** | Terminal - new session required |

### Implementation Files

```
src/chat/services/
├── state_machine/
│   ├── mod.rs              # Public exports
│   ├── state.rs            # ActorState enum
│   ├── event.rs            # ActorEvent enum
│   └── transition.rs       # Transition validation
├── states/
│   ├── idle.rs             # Idle state handler
│   ├── running.rs          # Running state handler
│   ├── paused.rs           # Paused state handler
│   ├── error.rs            # Error state (terminal)
│   ├── cancelled.rs        # Cancelled state (terminal)
│   └── completed.rs        # Completed state (terminal)
└── events/
    ├── process.rs          # ProcessInteraction handler
    ├── pause.rs            # Pause handler
    └── cancel.rs           # Cancel handler
```

---

## Agent Sessions

Agent sessions are persisted in the `agent_sessions` table, providing visibility, control, and monitoring.

### Session Lifecycle

```
┌─────────┐     ┌──────────┐     ┌────────┐     ┌───────────┐
│  Create │ ──> │   Idle   │ ──> │ Running│ ──> │ Completed │
└─────────┘     └──────────┘     └────────┘     └───────────┘
                     │                │
                     ▼                ▼
                  ┌────────┐     ┌────────┐
                  │ Paused │     │ Error  │
                  └────────┘     └────────┘
```

### Heartbeat Mechanism

- **Frequency**: Every 30 seconds while running
- **Threshold**: Sessions with heartbeat > 120 seconds are considered stale
- **Cleanup**: Automatic cleanup of stale sessions

### API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/workspaces/:id/agent-sessions` | List all active sessions |
| `GET /api/v1/agent-sessions/:id` | Get session details |
| `POST /api/v1/agent-sessions/:id/pause` | Pause session |
| `POST /api/v1/agent-sessions/:id/resume` | Resume session |
| `DELETE /api/v1/agent-sessions/:id` | Cancel session |

### ChatActor Integration

1. **Session Creation**: When actor starts, creates session record (status: `idle`)
2. **Status Updates**: State machine transitions are persisted
3. **Heartbeat**: Automatic heartbeat every 30 seconds while running
4. **Terminal States**: Actor shuts down, session preserved for audit
5. **Respawning**: New interactions spawn fresh actors hydrated from history

---

## Context Engineering

BuildScale treats LLM context as a dynamically engineered resource through a structured **BuiltContext** architecture.

### BuiltContext Struct

```rust
pub struct BuiltContext {
    /// System persona/instructions for the AI
    pub persona: String,
    /// History manager for conversation messages
    pub history: HistoryManager,
    /// Attachment manager for file attachments
    pub attachment_manager: AttachmentManager,
}
```

### AttachmentManager

Manages workspace file attachments with priority-based pruning:

```rust
pub struct AttachmentValue {
    pub content: String,
    pub priority: i32,      // Higher = dropped first during pruning
    pub tokens: usize,      // Estimated token count
    pub is_essential: bool, // Never prune if true
}
```

### Priority Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `PRIORITY_ESSENTIAL` | 0 | Never dropped |
| `PRIORITY_HIGH` | 3 | Dropped last |
| `PRIORITY_MEDIUM` | 5 | User attachments (default) |
| `PRIORITY_LOW` | 10 | Dropped first |

### Context Building Flow

1. **Load Messages**: Fetch all messages for chat from database
2. **Extract Persona**: Load from agents registry or chat config
3. **Split History**: Exclude last message, wrap in `HistoryManager`
4. **Hydrate Attachments**: Fetch file content, estimate tokens, add to manager
5. **Optimize**: Call `attachment_manager.optimize_for_limit()`
6. **Sort**: Sort attachments for consistent rendering order

### Security: Workspace Isolation

```rust
if file_with_content.file.workspace_id == workspace_id {
    attachment_manager.add_fragment(...);
}
// Files from other workspaces are silently ignored
```

---

## Context Caching

BuildScale leverages OpenAI and OpenRouter's **automatic prompt caching** through a cache-optimized context architecture.

### Cache-Optimized Architecture

```
[System Prompt] → [Interleaved History + Attachments] → [Last Message]
                            ↑                              ↑
                       cacheable                      varies only
```

### How It Works

1. **Attachment Timestamps**: Each attachment has `created_at` and `updated_at`
2. **Unified Context Items**: Messages and attachments wrapped in `ContextItem`
3. **Chronological Sorting**: All items sorted by timestamp (oldest first)
4. **Cache-Efficient Conversion**: Older items become stable prefix

### Tool Result Optimization

| Strategy | Value |
|----------|-------|
| Keep full outputs | Most recent 5 tool results |
| Truncate older results | 1KB with `…[re-run]` suffix |
| Tool calls | Always preserved |

### Example

```
Time 0:00 - File A attached
Time 0:05 - User message 1
Time 0:10 - Assistant response 1
Time 0:15 - File B attached
Time 0:20 - User message 2  ← Current prompt

Interleaved order (oldest first):
1. [Attachment: File A]     ← Cacheable
2. [User message 1]         ← Cacheable
3. [Assistant response 1]   ← Cacheable
4. [Attachment: File B]     ← Cacheable
5. [User message 2]         ← Varies
```

---

## AI Providers

BuildScale supports multiple AI provider integrations with granular control over model availability.

### Supported Providers

| Provider | Description |
|----------|-------------|
| **OpenAI** | GPT-4o, GPT-5-mini, etc. with reasoning summaries |
| **OpenRouter** | 200+ models through unified API |

### Configuration

```bash
# OpenAI Provider
BUILDSCALE__AI__PROVIDERS__OPENAI__API_KEY=sk-...
BUILDSCALE__AI__PROVIDERS__OPENAI__ENABLE_REASONING_SUMMARIES=true
BUILDSCALE__AI__PROVIDERS__OPENAI__REASONING_EFFORT=medium

# OpenRouter Provider
BUILDSCALE__AI__PROVIDERS__OPENROUTER__API_KEY=sk-or-...

# Default Provider
BUILDSCALE__AI__PROVIDERS__DEFAULT_PROVIDER=openai
BUILDSCALE__AI__PROVIDERS__DEFAULT_MODEL=openai:gpt-5-mini
```

### Model Identifier Format

| Format | Example |
|--------|---------|
| New (recommended) | `openai:gpt-4o`, `openrouter:anthropic/claude-3.5-sonnet` |
| Legacy | `gpt-4o` (uses default provider) |

### Database Schema

**ai_models table**: Stores all available models with global enable/disable control

| Column | Type | Description |
|--------|------|-------------|
| `provider` | TEXT | 'openai', 'openrouter' |
| `model_name` | TEXT | Model identifier |
| `is_enabled` | BOOLEAN | Globally enable/disable |

**workspace_ai_models table**: Controls workspace-model access

| Column | Type | Description |
|--------|------|-------------|
| `workspace_id` | UUID | Workspace reference |
| `model_id` | UUID | Model reference |
| `status` | TEXT | 'active', 'disabled', 'restricted' |

### API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/providers` | Get all configured providers |
| `GET /api/v1/workspaces/:id/providers` | Get workspace providers |

---

## Rig Integration

BuildScale integrates with the **Rig.rs** library for AI execution while maintaining separation of concerns.

### Separation of Concerns

| Responsibility | Component | Layer |
|----------------|-----------|-------|
| Persistent Memory | PostgreSQL | BuildScale OS |
| File Identity | `files` table | BuildScale OS |
| AI Reasoning | `rig::agent::Agent` | Rig Runtime |
| Tool Execution | `rig::tool::Tool` | Rig Runtime |

### Rig Prefix Mandate

All integration logic uses the `Rig` prefix:
- `RigLsTool`, `RigReadTool`, etc.
- `RigService`: Factory and orchestrator

### Tool Bridge

```rust
pub struct RigLsTool {
    pub conn: Arc<Mutex<DbConn>>,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

impl rig::tool::Tool for RigLsTool {
    type Error = Error;
    type Args = LsArgs;
    type Output = serde_json::Value;
    const NAME: &'static str = "ls";
}
```

### Chat History Management

Rig's `stream_chat` automatically maintains tool calls and results during streaming. For multi-turn conversations:

1. Tool calls/results persisted to database as `role: Tool` messages
2. `convert_history` reconstructs Rig messages from persisted data
3. Tool results stored as summaries to avoid storage bloat

---

## SSE Event Protocol

Standardized JSON payloads for UI rendering.

| Event Type | Data Payload | Purpose |
|------------|--------------|---------|
| `thought` | `{"text": "..."}` | AI reasoning stream |
| `call` | `{"tool": "write", "path": "..."}` | Tool invocation |
| `observation` | `{"output": "...", "success": true}` | Tool result |
| `file_updated` | `{"path": "...", "v": 4}` | UI refresh trigger |
| `plan_step` | `{"step": 3, "status": "done"}` | Progress update |
| `done` | `{"message": "..."}` | Completion |
| `stopped` | `{"reason": "...", "partial_response": "..."}` | Cancellation |
| `state_changed` | `{"from_state": "...", "to_state": "..."}` | State transition |

### State Machine SSE Events

**StateChanged Event:**
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

**Stopped Event (Terminal):**
```json
{
  "type": "stopped",
  "data": {
    "reason": "user_cancelled",
    "partial_response": "Text generated before stop..."
  }
}
```

---

## Related Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture overview
- [FILE_SYSTEM.md](./FILE_SYSTEM.md) - File system architecture
- [PLAN_SYSTEM.md](./PLAN_SYSTEM.md) - Plan mode and tools
- [API_REFERENCE.md](./API_REFERENCE.md) - Complete API reference
