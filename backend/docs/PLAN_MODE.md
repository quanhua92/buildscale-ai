# Plan Mode & Build Mode Specification

## 1. Overview
BuildScale AI implements a structured workflow that separates "Intent & Strategy" (Plan Mode) from "Execution & Implementation" (Build Mode). This ensures that the AI explores the project knowledge base and identifies all dependencies before modifying existing files.

## 2. Data Models

### 2.1 FileType::Plan
- **Extension**: `.plan` (e.g., `/plans/project-roadmap.plan`)
- **Nature**: A **Normal File** (not virtual). It exists in the standard filesystem and database registry.
- **Rust Enum**: Add `Plan` variant to `FileType` in `backend/src/models/files.rs`.

### 2.2 Chat Metadata (app_data)
The `.chat` virtual file tracks workflow state in its `app_data` column:
- `mode`: `plan` (default) or `build`.
- `plan_file`: The absolute path to the associated `.plan` file.
- **Persistence**: These fields are synchronized into the YAML frontmatter of the `.chat` file on disk for accessibility and readability.

## 3. Tool System Architecture

### 3.1 Universal System Tools
Beyond the file-specific tools, the system provides core tools available to all agents:
- **`ask_user`**: Suspends generation to request structured input or confirmation.
- **`exit_plan_mode`**: Transitions the workspace context from strategy (Plan) to implementation (Build).

### 3.2 Tool Configuration Struct
To ensure the `Tool` trait remains extensible without breaking every implementation, a `ToolConfig` struct is introduced.

```rust
pub struct ToolConfig {
    pub plan_mode: bool,
    pub active_plan_path: Option<String>,
    // Future fields: pub skills: Vec<String>, pub agent_id: Uuid, etc.
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            plan_mode: true, // Default to true for safety
            active_plan_path: None,
        }
    }
}
```

### 3.3 Tool Trait Refactor
The `Tool::execute` method is updated to receive this configuration.

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        config: ToolConfig, // Extensible config object
        args: Value,
    ) -> Result<ToolResponse>;
}
```

### 3.4 Guard & Enforcement Logic
- **Write/Edit Tools**: If `config.plan_mode` is `true`, these tools check the target file's type.
    - If `FileType` is `Plan`, the operation is **Allowed**.
    - For all other types, the operation is **Denied** with an error: *"System is in Plan Mode. You can only modify the plan. Switch to Build Mode to apply changes."*
- **Mkdir/Rm Tools**: Restricted to the `/plans/` directory in Plan Mode.

## 4. Agent Personas

### 4.1 Planner Agent (`backend/src/agents/planner.rs`)
- **System Prompt**: Focuses on project discovery (`grep`, `read`, `ls`) and strategic design.
- **Protocol**: Instructed to write its findings to a `/plans/*.plan` file and then ask the user for approval.
- **Exit Trigger**: Calls the `exit_plan_mode` tool when the strategy is finalized.

### 4.2 Builder Agent (`backend/src/agents/builder.rs`)
- **System Prompt**: Focuses on execution precision.
- **Context Injection**: `ChatService::build_context` detects the `plan_file` in metadata. It reads the plan's content from the filesystem and injects it into the LLM system prompt under a `## APPROVED EXECUTION PLAN` header.
- **Capability**: Full access to all tools (Plan Mode is `false`).

## 5. System Tool: `exit_plan_mode`
- **Arguments**: `plan_file_path: String`
- **Logic**:
    1. Verifies the existence of the plan file.
    2. Updates the chat session metadata in the database: `mode = build`, `plan_file = plan_file_path`.
    3. Updates the existing `FileVersion`'s `app_data` (mode: build, plan_file: path) and updates the `.chat` file's YAML frontmatter on disk, then touches the file.
    4. Emits an SSE event `ModeChanged` to signal the frontend.
    5. **Persona Shift**: The backend immediately swaps the active agent variant from `Planner` to `Builder`. The `Builder` persona contains persistent implementation instructions, removing the need for transient prompt injections.

## 6. Human-in-the-Loop: The Question Protocol

### 6.1 Overview

The Question Protocol enables AI agents to request structured input from users during execution. This human-in-the-loop mechanism uses **ephemeral questions** delivered via SSE events - no database or file storage required.

**Key Characteristics:**
- **Ephemeral**: Questions exist only in SSE stream and frontend memory
- **JSON Schema Driven**: Validation and UI generation from standard JSON Schema
- **No New Endpoints**: Uses existing `POST /messages` API for answers
- **Zero Database Changes**: Questions are not persisted
- **Batch Questions**: Can ask multiple questions in a single call (array of questions)

### 6.2 Tool: `ask_user`

The `ask_user` tool is available to all agents for requesting structured user input. Supports asking a single question or multiple questions in batch.

**Arguments (Single Question):**
```json
{
  "questions": [
    {
      "name": "confirm",
      "question": "Should I proceed with deleting the 15 temporary files?",
      "schema": {
        "type": "boolean",
        "description": "Choose true to proceed, false to cancel"
      },
      "buttons": [
        {"label": "Yes, proceed", "value": true, "variant": "primary"},
        {"label": "No, cancel", "value": false, "variant": "secondary"}
      ]
    }
  ]
}
```

**Arguments (Batch Questions):**
```json
{
  "questions": [
    {
      "name": "environment",
      "question": "Which deployment environment?",
      "schema": {"type": "string", "enum": ["dev", "staging", "prod"]},
      "buttons": [...]
    },
    {
      "name": "region",
      "question": "Which region?",
      "schema": {"type": "string", "enum": ["us-east-1", "eu-west-1", "ap-southeast-1"]}
    },
    {
      "name": "confirm",
      "question": "Confirm deployment?",
      "schema": {"type": "boolean"},
      "buttons": [...]
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "result": {
    "status": "question_pending",
    "question_id": "01234567-89ab-cdef-0123-456789abcdef"
  }
}
```

**Field Descriptions:**
- `questions` (required): Array of question objects - always an array (single question = 1-item array)
  - `name` (required): Identifier for the question (used in answer object)
  - `question` (required): Question text (Markdown)
  - `schema` (required): JSON Schema for answer validation and UI generation
  - `buttons` (optional): Array of button definitions (overrides schema-based rendering)
    - `label`: Button text
    - `value`: Answer value when clicked
    - `variant`: Visual style (`primary`, `secondary`, `danger`)

### 6.3 Question Types with JSON Schema

**Confirmation (Boolean):**
```json
{
  "questions": [
    {
      "name": "confirm",
      "question": "Should I proceed?",
      "schema": {"type": "boolean"},
      "buttons": [
        {"label": "Yes", "value": true},
        {"label": "No", "value": false}
      ]
    }
  ]
}
```

**Single Choice (String Enum):**
```json
{
  "questions": [
    {
      "name": "environment",
      "question": "Which deployment environment should I use?",
      "schema": {
        "type": "string",
        "enum": ["dev", "staging", "prod"],
        "description": "Select deployment target"
      },
      "buttons": [
        {"label": "Development", "value": "dev"},
        {"label": "Staging", "value": "staging"},
        {"label": "Production", "value": "prod", "variant": "danger"}
      ]
    }
  ]
}
```

**Text Input (Validated String):**
```json
{
  "questions": [
    {
      "name": "branch_name",
      "question": "What should I name the new feature branch?",
      "schema": {
        "type": "string",
        "pattern": "^[a-z0-9-]+$",
        "minLength": 3,
        "maxLength": 50,
        "description": "Branch name (lowercase, numbers, hyphens only)"
      }
    }
  ]
}
```

**Multi-Select (Array):**
```json
{
  "questions": [
    {
      "name": "components",
      "question": "Select which components to update:",
      "schema": {
        "type": "array",
        "items": {
          "type": "string",
          "enum": ["frontend", "backend", "database", "docs"]
        },
        "minItems": 1,
        "description": "Choose at least one component"
      }
    }
  ]
}
```

**Structured Input (Object):**
```json
{
  "questions": [
    {
      "name": "endpoint_config",
      "question": "Configure the new API endpoint:",
      "schema": {
        "type": "object",
        "properties": {
          "path": {
            "type": "string",
            "pattern": "^/[a-z0-9/-]+$"
          },
          "method": {
            "type": "string",
            "enum": ["GET", "POST", "PUT", "DELETE"]
          },
          "auth_required": {
            "type": "boolean",
            "default": true
          }
        },
        "required": ["path", "method"]
      }
    }
  ]
}
```

### 6.4 SSE Event: QuestionPending

When the AI calls `ask_user`, the backend emits a `QuestionPending` SSE event:

```typescript
{
  type: "question_pending",
  data: {
    question_id: string,      // UUID v7 (time-ordered)
    questions: Array<{        // Always an array (single question = 1-item array)
      name: string,           // Question identifier
      question: string,       // Markdown text
      schema: JSONSchema,     // For validation + UI generation
      buttons?: Array<{       // Optional UI override
        label: string,
        value: any,
        variant?: "primary" | "secondary" | "danger"
      }>
    }>,
    created_at: string        // ISO 8601 timestamp
  }
}
```

**Backend Emission:**
```rust
let question_id = Uuid::new_v7();
let event = SseEvent::QuestionPending {
    question_id,
    questions: args.questions.clone(),
    created_at: Utc::now().to_rfc3339(),
};
let _ = event_tx.send(event);
```

### 6.5 Frontend Processing

**Render Logic:**
```typescript
function renderQuestions(data: QuestionData) {
  return data.questions.map(q => ({
    name: q.name,
    question: q.question,
    element: q.buttons?.length > 0
      ? renderButtons(q)
      : renderSchemaForm(q.schema)
  }))
}

function renderSchemaForm(schema: JSONSchema) {
  switch (schema.type) {
    case 'boolean': return renderToggle(schema)
    case 'string':
      return schema.enum ? renderRadioGroup(schema.enum) : renderTextInput(schema)
    case 'array':
      return schema.items?.enum ? renderCheckboxGroup(schema.items.enum) : renderArrayInput(schema)
    case 'object': return renderObjectForm(schema.properties)
    default: return renderJsonEditor(schema)
  }
}
```

**UI Component Mapping:**
- `boolean` → Toggle switch or checkbox
- `string` → Text input (with `pattern`, `minLength`, `maxLength` validation)
- `string` + `enum` → Radio buttons or select dropdown
- `array` + `enum` → Checkbox group
- `object` → Nested form fields
- `array` → Tag input or JSON editor

**Answer Collection:**
- Frontend collects all answers into an object: `{name1: answer1, name2: answer2, ...}`
- Single question → `{confirm: true}`
- Batch questions → `{environment: "staging", region: "us-east-1", confirm: true}`

### 6.6 Answer Submission

**Use Existing API:**
```
POST /api/v1/workspaces/:workspace_id/chats/:chat_id/messages
```

**Request (Single Question):**
```json
{
  "content": "[Answered: Yes, proceed]",
  "metadata": {
    "question_answer": {
      "question_id": "uuid",
      "answers": {
        "confirm": true
      }
    }
  }
}
```

**Request (Batch Questions):**
```json
{
  "content": "[Deployment config: environment=staging, region=us-east-1, confirmed=true]",
  "metadata": {
    "question_answer": {
      "question_id": "uuid",
      "answers": {
        "environment": "staging",
        "region": "us-east-1",
        "confirm": true
      }
    }
  }
}
```

**No Backend Changes Required** - leverages existing message creation. The answer(s) are stored as a normal user message with metadata, so the AI naturally sees them in context.

**Frontend Submit Handler:**
```typescript
async function submitAnswers(questionId: string, answers: Record<string, any>) {
  const answerCount = Object.keys(answers).length

  const response = await fetch(
    `/api/v1/workspaces/${workspaceId}/chats/${chatId}/messages`,
    {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({
        content: answerCount === 1
          ? `[Answered: ${formatAnswer(Object.values(answers)[0])}]`
          : `[Answers: ${Object.entries(answers).map(([k,v]) => `${k}=${v}`).join(', ')}]`,
        metadata: {
          question_answer: {
            question_id: questionId,
            answers: answers
          }
        }
      })
    }
  )
  if (response.ok) {
    hideQuestionBar()
  }
}
```

### 6.7 State Management

**Backend (ChatActor):**
```rust
pub struct ChatActor {
    chat_id: Uuid,
    // No active question tracking needed - questions are ephemeral
}
```

**Ephemeral Design:**
- Questions exist only in SSE stream and frontend memory
- No in-memory tracking required
- On page reload → question lost (user can ask AI to repeat if needed)
- Multiple questions can be asked in batch via `questions` array

### 6.8 Edge Cases

| Scenario | Behavior |
|----------|----------|
| Page reload | Question lost (not persisted) |
| User sends other message | Old question ignored, AI processes new message |
| Network disconnect | Question lost, reconnect gets no `QuestionPending` |
| Invalid answer | Frontend validates against schema, returns 400 if invalid |
| User stops generation | Question bar disappears immediately |

### 6.9 Complete Flow Example

```
[User]: "Deploy the application"
    ↓
[AI]: "I need to know which environment to deploy to."
       Tool Call: ask_user({
         questions: [{
           name: "environment",
           question: "Which environment?",
           schema: {type: "string", enum: ["dev", "staging", "prod"]},
           buttons: [...]
         }]
       })
    ↓
[Backend]: Tool returns {status: "question_pending", question_id: "uuid"}
           Emits SSE: QuestionPending {...}
    ↓
[Frontend]: Shows question bar with 3 buttons
    ↓
[User]: Clicks "Staging"
    ↓
[Frontend]: POST /messages {
             content: "[Answered: staging]",
             metadata: {question_answer: {question_id: "uuid", answers: {environment: "staging"}}}
           }
    ↓
[Backend]: Creates User message (existing API)
           Triggers AI to process new message
    ↓
[AI]: Sees answer in chat history
       "Deploying to staging environment..."
       Proceeds with deployment
```

### 6.10 Plan Mode Integration

When used in Plan Mode, the `ask_user` tool enables the Planner to request approval before transitioning to Build Mode:

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

If the user clicks "Accept & Build":
1. Frontend sends answer via `POST /messages` with `{approve: true}`
2. Planner receives answer, calls `exit_plan_mode` tool
3. System transitions to Build Mode
4. Builder Agent continues execution with approved plan context

## 7. Frontend UI Design

### 7.1 Mode Select Toggle
- A tab-style toggle in the chat header: **[ PLAN ] [ BUILD ]**.
- Visual cues: Plan mode uses "Strategic Blue"; Build mode uses "Execution Green".
- Transition: Automatically flips when the `ModeChanged` SSE event is received.

### 7.2 Question Tool Bar
A dynamic interaction layer positioned above the chat input field.
- **Triggers**: Renders when the latest message in the history is a `ToolCall` for `ask_user`.
- **Action**: Buttons act as shortcuts that send a standard chat message containing the button's `value`.


## 8. Workflow Lifecycle
1. **New Chat**: Starts in **Plan Mode**. `planner.rs` is active.
2. **Exploration**: AI uses discovery tools to understand the project knowledge base.
3. **Drafting**: AI uses `write` to create `/plans/project-roadmap.plan`.
4. **Approval**: AI calls `ask_user` with the plan summary. User clicks **Accept & Build** in the Question Tool Bar.
5. **Transition**: Backend calls `exit_plan_mode`. UI flips to Build Mode.
6. **Execution**: `builder.rs` takes over. Plan content is pinned in the prompt. AI applies changes to project files.

## 9. Verification & Standards
- **Linter for Plans**: Plans should follow a markdown structure with checkboxes for tracking progress.
- **Safety Net**: If a user manually switches back to Plan Mode via UI, the `ToolConfig` must immediately reflect this in the next tool call to prevent accidental modifications.

---

## 10. Implementation Status (Backend)

### 10.1 Completed Phases

As of January 2025, the following phases have been implemented:

#### Phase 1: Data Model Foundation ✅
- `FileType::Plan` variant added to `backend/src/models/files.rs`
- Chat metadata extended with `mode` and `plan_file` fields
- Database migration created for new fields
- `ToolConfig` structure defined with extensibility support

#### Phase 2: Tool System Refactor ✅
- `Tool::execute` signature updated to accept `ToolConfig`
- Plan mode guards implemented in all file modification tools:
  - `write`, `edit`, `mv`, `touch`: Only allow `.plan` files in Plan Mode
  - `mkdir`, `rm`: Only allow `/plans/` directory operations in Plan Mode
- `ask_user` and `exit_plan_mode` system tools implemented

#### Phase 3: Handler Integration ✅
- New chats default to `mode="plan"` and `plan_file=null`
- ToolConfig derived from agent_config in `rig_engine.rs`
- Proper persona selection based on mode

#### Phase 4: AI Agent Personas ✅
- `backend/src/agents/planner.rs`: Strategic discovery persona
- `backend/src/agents/builder.rs`: Execution-focused persona with plan injection
- Mode-based persona selection in `agents/mod.rs`

#### Phase 5: AI Runtime Integration ✅
- All tools registered with Rig agent builder
- System tools (`ask_user`, `exit_plan_mode`) available to AI
- ToolConfig properly propagated through all tool adapters

#### Phase 6: YAML Frontmatter Sync ✅
- `backend/src/services/chat/sync.rs`: YAML serialization/deserialization
- `ChatFrontmatter` and `YamlFrontmatter` structs
- `ChatService::update_chat_metadata` for metadata updates
- `ChatService::sync_yaml_frontmatter` for file sync
- `ChatService::get_yaml_frontmatter` for debugging

#### Phase 7: Tool Migration ✅
- All 11 tools verified to have `ToolConfig` parameter
- All file modification tools have plan mode guards
- Comprehensive error messages for guard violations

#### Phase 8: Testing & Validation ✅
- Unit tests: `tests/tools/plan_mode_tests.rs` (3 tests)
- Unit tests: `tests/chat/yaml_sync_tests.rs` (6 tests)
- All 327 existing integration tests pass

#### Phase 9: Performance & Optimization ✅
- Database indexes created for mode/plan_file queries
- Edge case handling verified (missing metadata, deleted plans, etc.)

#### Phase 10: Documentation ✅
- Code docstrings added throughout
- System documentation updated
- Implementation notes added

### 10.2 File Locations

**Core Implementation:**
- `backend/src/models/files.rs` - FileType::Plan
- `backend/src/models/chat.rs` - AgentConfig with mode/plan_file
- `backend/src/tools/mod.rs` - ToolConfig definition
- `backend/src/tools/{write,edit,mv,rm,mkdir,touch}.rs` - Plan mode guards
- `backend/src/tools/ask_user.rs` - Universal system tool
- `backend/src/tools/exit_plan_mode.rs` - Mode transition tool
- `backend/src/agents/planner.rs` - Planner persona
- `backend/src/agents/builder.rs` - Builder persona
- `backend/src/services/chat/sync.rs` - YAML frontmatter sync
- `backend/src/services/chat/rig_engine.rs` - Agent builder with ToolConfig
- `backend/src/handlers/chat.rs` - Chat creation with default mode

**Tests:**
- `backend/tests/tools/plan_mode_tests.rs` - ToolConfig tests
- `backend/tests/chat/yaml_sync_tests.rs` - YAML sync tests

**Migrations:**
- `backend/migrations/20250131200000_plan_mode_indexes.up.sql` - Performance indexes

**Documentation:**
- `backend/docs/PLAN_MODE.md` - This file
- `backend/docs/TODO_PLAN_MODE.md` - Implementation checklist

### 10.3 Remaining Work (Frontend)

The backend implementation is complete. The following frontend work remains:

1. **Question Bar UI** - Render `ask_user` questions with buttons/form
2. **Mode Indicator** - Display current mode (Plan vs Build)
3. **YAML Frontmatter Display** - Parse and show chat metadata
4. **SSE Event Handling** - Handle `QuestionPending` and `ModeChanged` events
5. **Mode Toggle** - Manual mode switching via UI

### 10.4 API Contract

**Tool Execution (unchanged):**
```
POST /api/v1/workspaces/:id/tools
```

**Chat Messages (supports question answers):**
```
POST /api/v1/workspaces/:id/chats/:chat_id/messages

{
  "content": "[Answered: staging]",
  "metadata": {
    "question_answer": {
      "question_id": "uuid",
      "answers": {
        "environment": "staging"
      }
    }
  }
}
```

**SSE Events:**
```typescript
type QuestionPending = {
  type: "question_pending",
  data: {
    question_id: string,
    questions: Array<{
      name: string,
      question: string,
      schema: JSONSchema,
      buttons?: Array<{label, value, variant}>
    }>
  }
}

type ModeChanged = {
  type: "mode_changed",
  data: {
    mode: "plan" | "build",
    plan_file: string | null
  }
}
```
