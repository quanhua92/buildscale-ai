# Plan Mode & Plan Tools

BuildScale AI implements a structured workflow that separates "Intent & Strategy" (Plan Mode) from "Execution & Implementation" (Build Mode). This ensures that the AI explores the project knowledge base and identifies all dependencies before modifying existing files.

## Table of Contents

- [Overview](#overview)
- [Data Models](#data-models)
- [Tool System Architecture](#tool-system-architecture)
- [Agent Personas](#agent-personas)
- [Plan Management Tools](#plan-management-tools)
- [Human-in-the-Loop: The Question Protocol](#human-in-the-loop-the-question-protocol)
- [Workflow Lifecycle](#workflow-lifecycle)
- [Frontend UI Design](#frontend-ui-design)

---

## Overview

| Mode | Purpose | Allowed Actions |
|------|---------|-----------------|
| **Plan Mode** | Intent & Strategy | Read-only exploration + write to `/plans/*.plan` |
| **Build Mode** | Execution & Implementation | Full access to all tools |

Key benefits:
- AI explores project before modifying files
- User approval before implementation
- Clear separation of concerns
- Audit trail of decisions

---

## Data Models

### FileType::Plan

- **Extension**: `.plan` (e.g., `/plans/project-roadmap.plan`)
- **Nature**: A **Normal File** (not virtual). Exists in the standard filesystem and database registry.

### Chat Metadata (app_data)

The `.chat` virtual file tracks workflow state in its `app_data` column:

| Field | Values | Description |
|-------|--------|-------------|
| `mode` | `plan` (default) or `build` | Current workflow mode |
| `plan_file` | Absolute path | Path to the associated `.plan` file |

**Persistence**: These fields are synchronized into the YAML frontmatter of the `.chat` file on disk.

---

## Tool System Architecture

### Universal System Tools

Beyond the file-specific tools, the system provides core tools available to all agents:

| Tool | Description |
|------|-------------|
| `ask_user` | Suspends generation to request structured input or confirmation |
| `exit_plan_mode` | Transitions the workspace context from strategy (Plan) to implementation (Build) |

### Tool Configuration Struct

```rust
pub struct ToolConfig {
    pub plan_mode: bool,
    pub active_plan_path: Option<String>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            plan_mode: false, // Default to Build Mode for normal operation
            active_plan_path: None,
        }
    }
}
```

### Guard & Enforcement Logic

**Write/Edit Tools**: If `config.plan_mode` is `true`:
- If `FileType` is `Plan` → **Allowed**
- For all other types → **Denied** with error: *"System is in Plan Mode. You can only modify the plan. Switch to Build Mode to apply changes."*

**Mkdir/Rm Tools**: Restricted to the `/plans/` directory in Plan Mode.

---

## Agent Personas

### Planner Agent

- **System Prompt**: Focuses on project discovery (`grep`, `read`, `ls`) and strategic design
- **Protocol**: Writes findings to a `/plans/*.plan` file and asks user for approval
- **Exit Trigger**: Calls the `exit_plan_mode` tool when strategy is finalized

### Builder Agent

- **System Prompt**: Focuses on execution precision
- **Context Injection**: Reads plan content and injects it into the LLM system prompt under `## APPROVED EXECUTION PLAN`
- **Capability**: Full access to all tools (Plan Mode is `false`)

---

## Plan Management Tools

Specialized tools for managing plan files with automatic naming, YAML frontmatter, and metadata parsing.

### plan_write

Creates or updates a plan file with auto-generated name and YAML frontmatter.

**Arguments:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `title` | string | Yes | Title of the plan (shown in frontmatter) |
| `content` | string | Yes | Plan content in markdown |
| `path` | string | No | Custom path. Auto-generates if omitted |
| `status` | string | No | `draft` (default), `approved`, `implemented`, `archived` |

**Example:**
```json
{
  "title": "Feature Implementation Plan",
  "content": "# Overview\n\n## Goals\n- Goal 1\n- Goal 2",
  "status": "draft"
}
```

### plan_read

Reads a plan file with parsed frontmatter.

**Arguments:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | No | Full path to plan file |
| `name` | string | No | Plan name (without `.plan`), searches `/plans/` |
| `offset`, `limit`, `cursor` | various | No | Read parameters |

**Example:**
```json
{"name": "gleeful-tangerine-expedition"}
```

### plan_edit

Edits a plan file while preserving YAML frontmatter.

**Arguments:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | Path to plan file |
| `old_string`, `new_string` | string | For replace | Search and replace operation |
| `insert_line`, `insert_content` | various | For insert | Insert operation |
| `last_read_hash` | string | No | Hash from latest read (prevents conflicts) |

### plan_list

Lists all plan files with metadata.

**Arguments:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `status` | string | No | Filter by status |
| `limit` | integer | No | Maximum results (default: 50) |

### Plan File Format

```markdown
---
title: Feature Implementation Plan
status: draft
created_at: 2025-01-15T10:30:00Z
---

# Overview

Plan content goes here...
```

### Status Values

| Status | Description |
|--------|-------------|
| `draft` | Plan is being written/edited (default) |
| `approved` | Plan has been approved for implementation |
| `implemented` | Plan has been fully implemented |
| `archived` | Plan is no longer active |

### Auto-generated Names

When `path` is not provided to `plan_write`, a unique 3-word hyphenated name is automatically generated:
- `gleeful-tangerine-expedition`
- `calm-forest-symphony`
- `brave-mountain-voyage`

---

## Human-in-the-Loop: The Question Protocol

The Question Protocol enables AI agents to request structured input from users during execution using **ephemeral questions** delivered via SSE events.

### Key Characteristics

- **Ephemeral**: Questions exist only in SSE stream and frontend memory
- **JSON Schema Driven**: Validation and UI generation from standard JSON Schema
- **No New Endpoints**: Uses existing `POST /messages` API for answers
- **Zero Database Changes**: Questions are not persisted
- **Batch Questions**: Can ask multiple questions in a single call

### Tool: ask_user

**Arguments (Batch Questions):**
```json
{
  "questions": [
    {
      "name": "environment",
      "question": "Which deployment environment?",
      "schema": {"type": "string", "enum": ["dev", "staging", "prod"]},
      "buttons": [
        {"label": "Development", "value": "dev"},
        {"label": "Staging", "value": "staging"},
        {"label": "Production", "value": "prod", "variant": "danger"}
      ]
    },
    {
      "name": "confirm",
      "question": "Confirm deployment?",
      "schema": {"type": "boolean"},
      "buttons": [
        {"label": "Yes", "value": true},
        {"label": "No", "value": false}
      ]
    }
  ]
}
```

### Question Types with JSON Schema

| Type | Schema | UI Component |
|------|--------|--------------|
| Boolean | `{"type": "boolean"}` | Toggle switch or checkbox |
| String | `{"type": "string"}` | Text input |
| String Enum | `{"type": "string", "enum": [...]}` | Radio buttons or select |
| Array Enum | `{"type": "array", "items": {"enum": [...]}}` | Checkbox group |
| Object | `{"type": "object", "properties": {...}}` | Nested form fields |

### SSE Event: QuestionPending

```typescript
{
  type: "question_pending",
  data: {
    question_id: string,
    questions: Array<{
      name: string,
      question: string,
      schema: JSONSchema,
      buttons?: Array<{label, value, variant}>
    }>,
    created_at: string
  }
}
```

### Answer Submission

Uses existing API: `POST /api/v1/workspaces/:workspace_id/chats/:chat_id/messages`

```json
{
  "content": "[Answered: staging]",
  "metadata": {
    "question_answer": {
      "question_id": "uuid",
      "answers": {
        "environment": "staging",
        "confirm": true
      }
    }
  }
}
```

### Plan Mode Integration

In Plan Mode, `ask_user` enables the Planner to request approval:

```json
{
  "questions": [
    {
      "name": "approve",
      "question": "Review the implementation plan. Ready to proceed?",
      "schema": {"type": "boolean"},
      "buttons": [
        {"label": "Accept & Build", "value": true, "variant": "primary"},
        {"label": "Keep Planning", "value": false, "variant": "secondary"}
      ]
    }
  ]
}
```

### Tool: exit_plan_mode

**Arguments:** `plan_file_path: String`

**Logic:**
1. Verifies existence of plan file
2. Updates chat session metadata: `mode = build`, `plan_file = plan_file_path`
3. Updates YAML frontmatter on disk
4. Emits SSE event `ModeChanged`
5. **Persona Shift**: Swaps from Planner to Builder persona

---

## Workflow Lifecycle

```
1. New Chat
   └─> Starts in Plan Mode, planner.rs is active

2. Exploration
   └─> AI uses discovery tools (grep, read, ls) to understand project

3. Drafting
   └─> AI uses write to create /plans/project-roadmap.plan

4. Approval
   └─> AI calls ask_user with plan summary
   └─> User clicks "Accept & Build" in Question Bar

5. Transition
   └─> Backend calls exit_plan_mode
   └─> UI flips to Build Mode

6. Execution
   └─> builder.rs takes over
   └─> Plan content pinned in prompt
   └─> AI applies changes to project files
```

---

## Frontend UI Design

### Mode Select Toggle

- A tab-style toggle in the chat header: **[ PLAN ] [ BUILD ]**
- Visual cues: Plan mode uses "Strategic Blue"; Build mode uses "Execution Green"
- Transition: Automatically flips when `ModeChanged` SSE event is received

### Question Tool Bar

A dynamic interaction layer positioned above the chat input field:
- **Triggers**: Renders when latest message is a `ToolCall` for `ask_user`
- **Action**: Buttons send a standard chat message containing the button's `value`

---

## Implementation Status

### Completed (Backend)

| Phase | Status |
|-------|--------|
| Data Model Foundation | ✅ |
| Tool System Refactor | ✅ |
| Handler Integration | ✅ |
| AI Agent Personas | ✅ |
| AI Runtime Integration | ✅ |
| YAML Frontmatter Sync | ✅ |
| Tool Migration | ✅ |
| Testing & Validation | ✅ |
| Performance & Optimization | ✅ |
| Documentation | ✅ |

### File Locations

**Core Implementation:**
- `src/tools/plan/` - Plan tools (write, read, edit, list, ask_user, exit_plan_mode)
- `src/tools/mod.rs` - ToolConfig definition
- `src/agent/core/` - Planner and Builder personas
- `src/chat/services/sync.rs` - YAML frontmatter sync

### Frontend Work Remaining

1. **Question Bar UI** - Render `ask_user` questions
2. **Mode Indicator** - Display current mode
3. **YAML Frontmatter Display** - Parse and show chat metadata
4. **SSE Event Handling** - Handle `QuestionPending` and `ModeChanged`
5. **Mode Toggle** - Manual mode switching via UI

---

## Related Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture overview
- [FILE_SYSTEM.md](./FILE_SYSTEM.md) - File system architecture
- [AI_SYSTEM.md](./AI_SYSTEM.md) - AI agent system architecture
