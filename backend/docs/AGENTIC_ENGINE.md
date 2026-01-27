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

To ensure high performance without unlimited resource consumption, the **Agentic Engine** employs a stateful but ephemeral actor model.

### A. Self-Termination (The Idle Timeout)
Actors are not permanent. If a `ChatActor` receives no commands (interactions or pings) for a specific duration (default: **10 minutes**, configurable via `BUILDSCALE__AI__ACTOR_INACTIVITY_TIMEOUT_SECONDS`), it performs a graceful shutdown. 
*   **Idle Definition:** The timeout only counts down when the actor is **truly idle**. The timer is reset both at the start and the end of an interaction, ensuring the agent has a full window of life *after* completing its last tool call or thought.
*   **Logic:** A resettable inactivity timer monitors the `mpsc` command channel.
*   **Persistence:** Before shutting down, the actor ensures all state is flushed to the database.

### B. Persistent Event Bus (Stable SSE)
To prevent UI disruption during actor timeouts, the **Event Bus** is decoupled from the **Actor Task**.
*   **Registry Ownership:** The `AgentRegistry` maintains a persistent `broadcast` channel for every `chat_id`.
*   **Seamless Re-spawning:** When a user sends a message to an idle chat:
    1.  The Registry detects the actor is missing/dead.
    2.  A new `ChatActor` is spawned and "plugged into" the existing broadcast bus.
    3.  The UI (SSE) remains connected to the same bus throughout the transition, receiving the new response without needing a page refresh.

### C. Rehydration
When an actor is re-spawned, it "hydrates" its state by querying the latest message history and file registry, ensuring zero loss of context regardless of how many times the worker task has cycled.

## 4. Execution Scenarios & Workflows

### Scenario 1: chat_mode (Reactive Assistant)
*   **Behavior:** A standard 1:1 conversation. The agent executes tools (read/write) immediately after a user prompt and waits for the next turn.
*   **Focus:** Ad-hoc debugging, Q&A, and exploration.

### Scenario 2: plan_then_execute (Engineering Operator)
*   **Phase 1 (Planning):** If `plan_id` is null, a Planner Agent (often injected by the server) generates a detailed task list in a `/chats/plan.md` file.
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

## 6. Cancellation Protocol (Graceful Stop)

### A. Client-Side Stop Request
When user presses STOP, the frontend sends a cancellation command:
**Endpoint:** `POST /api/v1/workspaces/:id/chats/:chat_id/stop`
**Response:** `200 OK` with `{ "status": "cancelled", "chat_id": "..." }`

### B. Graceful Cancellation Behavior
- **Tool Completion**: If AI is executing a tool (e.g., `write_file`), the tool completes first
- **Partial Save**: Any text generated before cancellation is saved to `chat_messages` table
- **System Marker**: A system message is added indicating user cancelled for AI context
- **Actor Continues**: The actor remains alive for future interactions (only resets interaction state)

### C. Cancellation Flow
1. Frontend calls `POST /chats/{id}/stop`
2. Backend sends `AgentCommand::Cancel` to actor
3. Actor sets `CancellationToken` (cooperative cancellation)
4. Streaming loop checks token after each event/chunk
5. If mid-tool-call, waits for tool result before stopping
6. Sends `SseEvent::Stopped` event to all clients
7. Saves partial response (if any text generated)
8. Adds system marker: `[System: Response was interrupted by user (user_cancelled)]`
9. Actor continues running, ready for next interaction

### D. Error Handling
- **Actor Not Found**: Returns `404 Not Found` if chat doesn't exist or actor timed out
- **Multiple Stops**: Idempotent - second stop request is harmless
- **No Partial Text**: Only saves assistant message if text was generated

## 7. Infrastructure: The Sandbox Hydration
To maintain security while allowing high-performance execution (e.g., npm, cargo, python), BuildScale uses a hydration pattern:

*   **Spin Up:** A Docker sandbox starts on the same host as the NVMe mirror.
*   **Hydrate:** The workspace (or a specific "slice") is mounted to the container as read-only.
*   **Agentic Engine Write:** The AI runs scripts at native hardware speeds. Any permanent file changes must be sent back through the **Agentic Engine's** write tool, ensuring every global state change is versioned and audited.

## 8. Conclusion
By treating Identity as a File and Memory as a Mirror, BuildScale.ai creates a future-proof environment. Whether for a developer refactoring an **Agentic Engine** or a blogger orchestrating a marketing team, the system provides a single, **Agentic Engine** source of truth for all human-AI collaboration.
