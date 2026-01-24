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
    { "file_id": "uuid-requirements.pdf" },
    { "file_id": "uuid-schema-draft.sql" }
  ],
  "agents": [
    { "role": "Architect", "agent_id": "arch-v1" }
  ],
  "config": { "mode": "plan_then_execute" }
}
```

### B. Server Handshake (SSE Initialization)
The server returns an SSE stream. The first event (`session_init`) provides the file anchors.
*   **chat_id:** The UUID of the **Agentic Engine** `.chat` file.
*   **plan_id:** The UUID of the **Agentic Engine** `.plan.md` file.
*   **agents:** The final list of active agents (including server-injected agents like a Planner).

### C. Subsequent Requests (The Follow-up)
Once anchored, the client sends only the newest interaction. The **Agentic Engine** reads the chat file to reconstruct the deep history.
**Endpoint:** `POST /api/v1/workspaces/:workspace_id/chats/:chat_id`

```json
{
  "chat_id": "uuid-chat-file",
  "plan_id": "uuid-plan-file",
  "recent_messages": [
    { "role": "user", "content": "Actually, use Stripe for the billing." }
  ]
}
```

## 3. Execution Scenarios & Workflows

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

## 4. The SSE Event Protocol
The UI renders the **Agentic Engine's** internal actions using standardized JSON payloads.

| Event Type | Data Payload | Purpose |
| :--- | :--- | :--- |
| **thought** | `{"agent_id": "...", "text": "..."}` | Streams the "Internal Monologue" of an agent. |
| **call** | `{"tool": "write", "path": "..."}` | Logs a literal **Agentic Engine** disk action. |
| **observation** | `{"output": "..."}` | The result of the tool (e.g., shell output, file content). |
| **file_updated** | `{"path": "/src/auth.rs", "v": 4}` | Triggers immediate UI refresh (Explorer/Editor). |
| **plan_step** | `{"step": 3, "status": "done"}` | Updates the task checklist/progress UI. |
| **done** | `{"message": "Task complete."}` | Finalizes the execution turn. |

## 5. Infrastructure: The Sandbox Hydration
To maintain security while allowing high-performance execution (e.g., npm, cargo, python), BuildScale uses a hydration pattern:

*   **Spin Up:** A Docker sandbox starts on the same host as the NVMe mirror.
*   **Hydrate:** The workspace (or a specific "slice") is mounted to the container as read-only.
*   **Agentic Engine Write:** The AI runs scripts at native hardware speeds. Any permanent file changes must be sent back through the **Agentic Engine's** write tool, ensuring every global state change is versioned and audited.

## 6. Conclusion
By treating Identity as a File and Memory as a Mirror, BuildScale.ai creates a future-proof environment. Whether for a developer refactoring an **Agentic Engine** or a blogger orchestrating a marketing team, the system provides a single, **Agentic Engine** source of truth for all human-AI collaboration.
