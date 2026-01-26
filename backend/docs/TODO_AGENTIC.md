# Implementation Plan: Agentic Engine (The Brain)

**Status**: 🚧 In Progress
**Architecture Reference**: `backend/docs/AGENTIC_ENGINE.md`

This document serves as the self-contained execution guide for implementing the Agentic Engine in BuildScale.ai. It focuses on delivering a rapid MVP (Reactive Chat) before expanding into complex autonomous capabilities.

---

## 📅 Progress Log

- [x] **Phase 1.1**: Data Structures & Schema (Committed)
- [x] **Phase 1.1b**: Data Access Layer (Committed)
- [x] **Phase 1.2**: Context Construction & Orchestration (Committed)
- [x] **Phase 1.3**: Rig Integration & Execution Runtime (Committed)
- [x] **Phase 1.4**: Decoupled Command/Event Loop (Committed)
- [x] **Phase 2**: Tool Execution (Committed)

---

## Phase 1: MVP Chat Mode (The Reactive Core)
**Goal**: Implement a standard chat interface where the user can select an Agent, attach Files, and have a conversation.
**Architecture**: We will use the **Virtual File** pattern (enabled by the `is_virtual` flag in the `files` table).
*   **Storage**: Messages are stored in a high-performance `chat_messages` append-only table.
*   **Projection**: The system exposes a `.chat` file identity, but its content is dynamically assembled from the table when read. This avoids creating a full `file_version` blob for every message turn.

- [x] **1.1 Data Structures & Schema**
    - [x] Create migration for `chat_messages` table (id, file_id, role, content, created_at).
    - [x] Define `ChatSession` struct (`src/models/chat.rs`).
    - [x] Define `ChatMessage` struct (User/Assistant/System roles).
    - [x] Define `AgentConfig` struct (Persona definition).
- [x] **1.1b Data Access Layer (Dumb Queries)**
    - [x] Implement `insert_chat_message` (`src/queries/chat.rs`).
    - [x] Implement `get_messages_by_file_id` (Retrieval).
    - [x] Implement `update_chat_message` (Editing).
    - [x] Implement `soft_delete_chat_message` (Lifecycle).
    - [x] Implement `touch_file` query (`src/queries/files.rs`).
- [x] **1.2 Context Construction (Service Layer)**
    - [x] `build_system_prompt(agent_id)`: Load the Agent's specific instruction file from `/system/agents/`.
    - [x] `hydrate_context(file_ids)`: Read content of attached files and format for the LLM.
    - [x] `get_chat_history(chat_file_id)`: Query `chat_messages` table.
- [x] **1.3 Rig Integration & Execution Runtime (`src/services/chat/rig_*`)**
    - [x] Integrate `rig-core` and `rig-openai`.
    - [x] Implement `RigTool` wrappers for native filesystem tools (`ls`, `read`, `write`, `rm`).
    - [x] Implement `RigService` for agent orchestration and history conversion.
- [x] **1.4 Decoupled Command/Event Loop (`src/handlers/chat.rs`)**
    - [x] `POST /api/v1/workspaces/:id/chats`: Initialize session (Create `.chat` file with `is_virtual: true`).
    - [x] `GET /api/v1/workspaces/:id/chats/:chat_id/events`: The Event Pipe (SSE stream from `ChatActor`).
    - [x] `POST /api/v1/workspaces/:id/chats/:chat_id`: The Command Bus (Appends message and signals actor).
    - [x] `ChatActor`: Background worker for autonomous execution and persistence.

## Phase 2: Tool Execution (The Hands)
**Goal**: Allow the LLM to call `read`, `write`, `ls` commands to interact with the environment.

- [x] **2.1 Tool Definitions**
    - [x] Define JSON schemas for `read`, `write`, `ls`, `rm` via `schemars`.
    - [x] Inject schemas into Rig tools via `definition()` method.
- [x] **2.2 Parser & Dispatcher**
    - [x] Rig natively parses tool calls from LLM stream.
    - [x] `RigTool` implementation dispatches to native OS tools.
- [x] **2.3 The Observation Loop**
    - [x] Rig handles feeding tool outputs back to LLM.
    - [x] `ChatActor` streams `thought`, `call`, and `observation` events to SSE.

## Phase 3: The Planner (The Cortex)
**Goal**: Implement the "Plan -> Execute" workflow using `.plan.md` files.

- [ ] **3.1 Plan Schema**
    - [ ] Define structure of `.plan.md` (Checklist format).
- [ ] **3.2 Planner Agent**
    - [ ] specialized prompt to generate plans instead of code.
- [ ] **3.3 Execution Loop**
    - [ ] Logic to read plan, pick next step, execute, and update plan file.

## Phase 4: Autonomous Loop (The Agent)
**Goal**: Background execution without user interaction.

- [ ] **4.1 Background Worker**
    - [ ] Detach execution from HTTP request.
    - [ ] Re-attach logic for User UI to "catch up" via log reading.

---

## 🏁 Definition of Done (MVP)
1.  Can start a chat with a specific Agent role (e.g., `assistant`).
2.  Can reference specific files in the workspace (read-only context).
3.  Responses and interleaved tool calls are streamed via SSE.
4.  Conversation history is persisted to `.chat` files.
