# Implementation Plan: Agentic Engine (The Brain)

**Status**: 🚧 In Progress
**Architecture Reference**: `backend/docs/AGENTIC_ENGINE.md`

This document serves as the self-contained execution guide for implementing the Agentic Engine in BuildScale.ai. It focuses on delivering a rapid MVP (Reactive Chat) before expanding into complex autonomous capabilities.

---

## 📅 Progress Log

- [ ] **Phase 1**: MVP Chat Mode (The Reactive Core)
- [ ] **Phase 2**: Tool Execution (The Hands)
- [ ] **Phase 3**: The Planner (The Cortex)
- [ ] **Phase 4**: Autonomous Loop (The Agent)

---

## Phase 1: MVP Chat Mode (The Reactive Core)
**Goal**: Implement a standard chat interface where the user can select an Agent, attach Files, and have a conversation.
**Architecture**: We will use the **Virtual File** pattern (enabled by the `is_virtual` flag in the `files` table).
*   **Storage**: Messages are stored in a high-performance `chat_messages` append-only table.
*   **Projection**: The system exposes a `.chat` file identity, but its content is dynamically assembled from the table when read. This avoids creating a full `file_version` blob for every message turn.

- [ ] **1.1 Data Structures & Schema**
    - [ ] Create migration for `chat_messages` table (id, file_id, role, content, created_at).
    - [ ] Define `ChatSession` struct (`src/models/chat.rs`).
    - [ ] Define `ChatMessage` struct (User/Assistant/System roles).
    - [ ] Define `AgentConfig` struct (Persona definition).
- [ ] **1.2 Context Construction (Service Layer)**
    - [ ] `build_system_prompt(agent_id)`: Load the Agent's specific instruction file from `/system/agents/`.
    - [ ] `hydrate_context(file_ids)`: Read content of attached files and format for the LLM.
    - [ ] `get_chat_history(chat_file_id)`: Query `chat_messages` table.
- [ ] **1.3 The LLM Client (`src/services/llm.rs`)**
    - [ ] Implement generic Client trait (OpenAI compatible).
    - [ ] Implement `stream_completion` method supporting SSE.
- [ ] **1.4 The SSE Endpoint (`src/handlers/chat.rs`)**
    - [ ] `POST /api/v1/workspaces/:id/chats`: Initialize session (Create `.chat` file with `is_virtual: true`).
    - [ ] `POST /api/v1/workspaces/:id/chats/:chat_id`: The Follow-up Loop.
        1.  User sends message -> Insert into `chat_messages`.
        2.  Server rebuilds full context (System + Files + History + User Msg).
        3.  Server calls LLM Stream.
        4.  Server pushes SSE events (`thought`, `chunk`, `done`).
        5.  Server appends Assistant message -> Insert into `chat_messages`.

## Phase 2: Tool Execution (The Hands)
**Goal**: Allow the LLM to call `read`, `write`, `ls` commands to interact with the environment.

- [ ] **2.1 Tool Definitions**
    - [ ] Define JSON schemas for `read`, `write`, `ls`, `grep`.
    - [ ] Inject schemas into LLM context.
- [ ] **2.2 Parser & Dispatcher**
    - [ ] Parse tool calls from LLM stream.
    - [ ] Dispatch to `FilesService` (reusing logic from File System phases).
- [ ] **2.3 The Observation Loop**
    - [ ] Feed tool outputs back to LLM as a new "User" or "Tool" message.
    - [ ] Recursively call LLM until final answer.

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
1.  Can start a chat with a specific Agent persona.
2.  Can reference specific files in the workspace (read-only context).
3.  Responses are streamed via SSE.
4.  Conversation history is persisted to `.chat` files.
