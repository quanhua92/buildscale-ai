← [Back to Index](./README.md) | **Related**: [Files Are All You Need](./FILES_ARE_ALL_YOU_NEED.md)

# BuildScale.ai | The Agentic Engine Specification

This document serves as the **Agentic Engine** specification for BuildScale.ai, a Distributed Operating System where the backend acts as a high-performance **Agentic Engine**. It replaces the traditional "AI-as-a-Proxy" model with a stateful environment where AI agents live, plan, and execute directly within an **Agentic Engine** workspace mirror.

## 1. Core Philosophy: The Agentic Engine

BuildScale.ai is built on the principle that The Workspace is the OS.

*   **Agentic Engine Authority:** The Rust backend owns the "hands" (Tools API) and the "memory" (File Registry). The LLM is merely a swappable "processor".
*   **Identity vs. Content:** Every object (code, chat, plan, agent) has a permanent Identity in the registry (id, path) and an immutable Content history (file_versions).
*   **The Write-Flush Loop:** Every AI action (e.g., `write_file`) is committed to PostgreSQL and physically flushed to the Local NVMe Mirror before the agent's next "thought," ensuring zero state drift.

## 2. The REST API: Handshake & State Management

The API manages sessions as persistent files. The first request initializes the environment; subsequent requests send only the "delta".

### A. Initial Request (The Seed)
Used to start a conversation, attach files, or define a goal.
**Endpoint:** `POST /api/v1/workspaces/:id/chats`

```json
{
  "goal": "Build a multi-tenant subscription system.",
  "files": [
    "019bf537-f228-7cd3-aa1c-3da8af302e12",
    "019bf537-f239-7122-a146-af0ff3438892"
  ],
  "role": "assistant",
  "model": "gpt-4o-mini"
}
```

### B. Server Handshake (The Event Pipe)
The client connects to the event stream to receive real-time updates. This pipe is decoupled from commands to allow background execution.
**Endpoint:** `GET /api/v1/workspaces/:id/chats/:chat_id/events`
*   **Response:** `text/event-stream`
*   **Events:** `thought`, `call`, `observation`, `chunk`, `done`.

### C. Subsequent Requests (The Command Bus)
Once anchored, the client sends new interactions via standard POST requests. The backend processes these in a persistent background actor.
**Endpoint:** `POST /api/v1/workspaces/:workspace_id/chats/:chat_id`
*   **Response:** `202 Accepted`

## 3. Actor Lifecycle & Resource Management

To ensure high performance without unlimited resource consumption, the **Agentic Engine** employs a stateful but ephemeral actor model with an explicit state machine.

### A. State Machine Architecture

The `ChatActor` uses a finite state machine (FSM) for clear, explicit state management:

- **Idle**: Agent is waiting for user input or events
- **Running**: Agent is actively processing an interaction
- **Paused**: Agent is temporarily paused
- **Error**: Terminal state - agent encountered an error
- **Cancelled**: Terminal state - agent was cancelled by user
- **Completed**: Terminal state - agent completed naturally (timeout)

**Terminal States**: Once an agent enters `Error`, `Cancelled`, or `Completed`:
- No further state transitions are possible
- The actor automatically shuts down
- The session is preserved in the database for audit/history
- A new interaction spawns a fresh actor

See [Agent State Machine](./AGENT_STATE_MACHINE.md) for complete state machine specification.

### B. Self-Termination (The Idle Timeout)
Actors are not permanent. If a `ChatActor` receives no commands (interactions or pings) for a specific duration (default: **10 minutes**, configurable via `BUILDSCALE__AI__ACTOR_INACTIVITY_TIMEOUT_SECONDS`), it transitions to `Completed` (terminal) and shuts down.
*   **Idle Definition:** The timeout only counts down when the actor is **truly idle**. The timer is reset both at the start and the end of an interaction, ensuring the agent has a full window of life *after* completing its last tool call or thought.
*   **Logic:** A resettable inactivity timer monitors the `mpsc` command channel.
*   **Persistence:** Before shutting down, the actor ensures all state is flushed to the database.

### C. Persistent Event Bus (Stable SSE)
To prevent UI disruption during actor timeouts, the **Event Bus** is decoupled from the **Actor Task**.
*   **Registry Ownership:** The `AgentRegistry` maintains a persistent `broadcast` channel for every `chat_id`.
*   **Seamless Re-spawning:** When a user sends a message to an idle chat:
    1.  The Registry detects the actor is missing/dead.
    2.  A new `ChatActor` is spawned and "plugged into" the existing broadcast bus.
    3.  The UI (SSE) remains connected to the same bus throughout the transition, receiving the new response without needing a page refresh.

### D. Rehydration
When an actor is re-spawned, it "hydrates" its state by querying the latest message history and file registry, ensuring zero loss of context regardless of how many times the worker task has cycled.

## 4. Execution Scenarios & Workflows

### Scenario 1: chat_mode (Reactive Assistant)
*   **Behavior:** A standard 1:1 conversation. The agent executes tools (read/write) immediately after a user prompt and waits for the next turn.
*   **Focus:** Ad-hoc debugging, Q&A, and exploration.

### Scenario 2: plan_then_execute (Engineering Operator)
*   **Phase 1 (Planning):** If `plan_id` is null, a Planner Agent (often injected by the server) generates a detailed task list in a `/plans/*.plan` file.
*   **Phase 2 (Execution):** Once approved, a Coder Agent follows the Markdown checklist, committing each change as a version and flushing to disk.

### Scenario 3: parallel_agents (The Matrix)
*   **Behavior:** Multiple agents (e.g., Frontend Dev, Backend Dev, Reviewer) work on the same plan simultaneously.
*   **Synchronization:** They coordinate through the Plan File. Since every write is flushed to the NVMe mirror immediately, parallel agents see updated code in real-time without collisions.

### Scenario 4: autonomous (Background Worker)
*   **Behavior:** The agent runs unsupervised until a goal is met or it hits a limit.
*   **Resilience:** If the user closes the UI, the Rust backend continues the loop. Upon reconnecting with the `chat_id`, the user "catches up" by reading the **Agentic Engine** chat file logs.

### Scenario 5: Non-Developer Co-work (The Agentic CMS)
*   **Workflow:** A Blogger (Non-Developer) wants to launch a content campaign.
*   **Orchestration:**
    *   User selects a "Content Strategist" agent and provides a "Topic."
    *   The server injects a "Researcher" and a "SEO Specialist."
    *   The agents build a plan in `/content/campaign-plan.md`.
    *   The user views the plan as a high-level dashboard. As the agents write to the Headless CMS, the user sees progress bars move in real-time via the SSE stream.

## 5. The SSE Event Protocol
The UI renders the **Agentic Engine's** internal actions using standardized JSON payloads.

| Event Type | Data Payload | Purpose |
| :--- | :--- | :--- |
| **thought** | `{"text": "..."}` | Streams the "Internal Monologue" of an agent. |
| **call** | `{"tool": "write", "path": "..."}` | Logs a literal **Agentic Engine** disk action. |
| **observation** | `{"output": "...", "success": true}` | The result of the tool (e.g., shell output, file content). |
| **file_updated** | `{"path": "/src/auth.rs", "v": 4}` | Triggers immediate UI refresh (Explorer/Editor). |
| **plan_step** | `{"step": 3, "status": "done"}` | Updates the task checklist/progress UI. |
| **done** | `{"message": "Task complete."}` | Finalizes the execution turn. |
| **stopped** | `{"reason": "...", "partial_response": "..."}` | Signals graceful cancellation to UI. |
| **state_changed** | `{"from_state": "idle", "to_state": "running", "reason": "..."}` | Signals state machine transition. |

## 6. Cancellation Protocol (Graceful Stop)

### A. Client-Side Stop Request
When user presses STOP, the frontend sends a cancellation command:
**Endpoint:** `POST /api/v1/workspaces/:id/chats/:chat_id/stop`
**Response:** `200 OK` with `{ "status": "cancelled", "chat_id": "..." }`

### B. Graceful Cancellation Behavior
- **Tool Completion**: If AI is executing a tool (e.g., `write_file`), the tool completes first
- **Partial Save**: Any text generated before cancellation is saved to `chat_messages` table
- **System Marker**: A system message is added indicating user cancelled for AI context
- **Terminal State**: Actor enters `Cancelled` (terminal) state and shuts down
- **Session Preserved**: The session record remains in database with status `cancelled` for audit/history

### C. Cancellation Flow
1. Frontend calls `POST /chats/{id}/stop`
2. Backend sends `AgentCommand::Cancel` to actor
3. Actor sets `CancellationToken` (cooperative cancellation)
4. Streaming loop checks token after each event/chunk
5. If mid-tool-call, waits for tool result before stopping
6. Sends `SseEvent::Stopped` event to all clients
7. Saves partial response (if any text generated)
8. Adds system marker: `[System: Response was interrupted by user (user_cancelled)]`
9. Actor transitions to `Cancelled` (terminal) state
10. Actor shuts down (command channel closes)
11. Session persists in database with status `cancelled`

### D. Error Handling
- **Actor Not Found**: Returns `404 Not Found` if chat doesn't exist or actor timed out
- **Already Cancelled**: Returns `200 OK` with idempotent behavior if session already in `cancelled` terminal state
- **No Partial Text**: Only saves assistant message if text was generated

### E. Restarting After Cancellation
- Sending a new message after cancellation spawns a **fresh actor**
- The new actor is hydrated from the chat history
- Previous cancelled session is preserved for audit
- The new session gets a new session record in the database

## 7. Message Persistence & Audit Trail

All streaming events are now persistently stored in the `chat_messages` table, providing a complete audit trail:

| Event Type | Persisted As | Metadata |
|------------|--------------|----------|
| `thought` (reasoning chunks) | `ChatMessage` with `role=Assistant`, `message_type="reasoning_complete"` (Aggregated) | `reasoning_id` (UUID groups chunks) |
| `call` (tool invocation) | `ChatMessage` with `role=Tool`, `message_type="tool_call"` | `tool_name`, `tool_arguments` |
| `observation` (tool result) | `ChatMessage` with `role=Tool`, `message_type="tool_result"` | `tool_name`, `tool_output`, `tool_success` |

This ensures that when a chat is reopened, the full interaction history—including AI reasoning, tool calls, and results—is available for audit, debugging, and compliance.

Reasoning chunks are buffered and saved as a single aggregated message per turn to optimize storage, linked by `reasoning_id`. Frontends may collapse these by default.

See `docs/CHAT_PERSISTENCE_AUDIT.md` for the full specification.

## 8. Infrastructure: The Sandbox Hydration
To maintain security while allowing high-performance execution (e.g., npm, cargo, python), BuildScale uses a hydration pattern:

*   **Spin Up:** A Docker sandbox starts on the same host as the NVMe mirror.
*   **Hydrate:** The workspace (or a specific "slice") is mounted to the container as read-only.
*   **Agentic Engine Write:** The AI runs scripts at native hardware speeds. Any permanent file changes must be sent back through the **Agentic Engine's** write tool, ensuring every global state change is versioned and audited.

## 9. Conclusion
By treating Identity as a File and Memory as a Mirror, BuildScale.ai creates a future-proof environment. Whether for a developer refactoring an **Agentic Engine** or a blogger orchestrating a marketing team, the system provides a single, **Agentic Engine** source of truth for all human-AI collaboration.
