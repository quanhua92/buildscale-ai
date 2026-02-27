# System Architecture

Multi-tenant workspace-based RBAC architecture with clear separation of concerns

## Overview

**Core Architecture**: Multi-tenant Rust backend implementing workspace-based isolation with role-based access control (RBAC).

**Key Characteristics**:
- **Workspace Isolation**: Complete data separation between workspaces
- **RBAC System**: Four-tier role hierarchy (Admin > Editor > Member > Viewer)
- **Single Owner Model**: Each workspace has exactly one owner
- **Flexible Membership**: Users can belong to multiple workspaces with different roles
- **Comprehensive Permission System**: Fine-grained permissions across workspace, content, and member management categories
- **Layered Module Architecture**: Clear separation of concerns (Models -> Queries -> Services -> Handlers)

## Module Structure

```
src/
├── lib.rs           # Public exports and configuration loading
├── main.rs          # Application entry point
├── config.rs        # Environment configuration with BUILDSCALE_ prefix
├── database.rs      # Database connection pooling
├── error.rs         # Comprehensive error handling
├── validation.rs    # Input validation utilities (email, password, workspace names, etc.)
├── state.rs         # Application state management
│
├── agent/           # Agent module (AI personas and session management)
│   ├── mod.rs       # Module exports and get_persona() function
│   ├── core/        # Agent personas (system prompts)
│   │   ├── mod.rs
│   │   ├── common.rs    # Shared persona utilities
│   │   ├── assistant.rs # Default assistant persona
│   │   ├── planner.rs   # Strategic planning persona
│   │   └── builder.rs   # Plan execution persona
│   ├── models/      # Data structures
│   │   ├── mod.rs
│   │   └── session.rs   # AgentSession, AgentType, SessionStatus
│   ├── queries/     # Database operations
│   │   ├── mod.rs
│   │   └── sessions.rs  # Agent session CRUD
│   ├── services/    # Business logic
│   │   ├── mod.rs
│   │   └── sessions.rs  # Session lifecycle management
│   └── handlers/    # HTTP API endpoints
│       ├── mod.rs
│       └── sessions.rs  # REST handlers for sessions
│
├── chat/            # Chat module (AI conversations)
│   ├── mod.rs       # Module exports
│   ├── models/      # Data structures
│   │   ├── mod.rs
│   │   └── message.rs   # ChatMessage, NewChatMessage, ChatAttachment
│   ├── queries/     # Database operations
│   │   ├── mod.rs
│   │   └── messages.rs  # Message CRUD operations
│   ├── services/    # Business logic (complex subsystem)
│   │   ├── mod.rs       # ChatService, BuiltContext, constants
│   │   ├── context.rs   # AttachmentManager, HistoryManager
│   │   ├── sync.rs      # YAML frontmatter sync, ChatFrontmatter
│   │   ├── registry.rs  # Chat session registry
│   │   ├── actor/       # Actor-based session management
│   │   │   ├── mod.rs
│   │   │   ├── actor.rs     # ChatActor implementation
│   │   │   ├── state.rs     # Actor state management
│   │   │   ├── stream.rs    # SSE streaming
│   │   │   ├── session.rs   # Session lifecycle
│   │   │   ├── interaction.rs # ProcessorContext
│   │   │   ├── constants.rs # Actor constants
│   │   │   └── state_machine.rs # Actor state machine
│   │   ├── engine/      # Rig AI engine integration
│   │   │   ├── mod.rs
│   │   │   ├── rig.rs       # RigService, agent creation
│   │   │   └── tools.rs     # Tool adapters for Rig
│   │   ├── state_machine/ # Chat state machine
│   │   │   ├── mod.rs
│   │   │   ├── machine.rs   # State machine core
│   │   │   ├── state.rs     # State definitions
│   │   │   ├── event.rs     # Event definitions
│   │   │   └── transition.rs # State transitions
│   │   ├── states/      # State implementations
│   │   │   ├── mod.rs
│   │   │   ├── idle.rs      # Idle state
│   │   │   ├── running.rs   # Running state
│   │   │   ├── paused.rs    # Paused state
│   │   │   ├── completed.rs # Completed state
│   │   │   ├── cancelled.rs # Cancelled state
│   │   │   ├── error.rs     # Error state
│   │   │   └── shutdown.rs  # Shutdown state
│   │   └── events/      # Event processors
│   │       ├── mod.rs
│   │       ├── process.rs   # Process event
│   │       ├── pause.rs     # Pause event
│   │       ├── cancel.rs    # Cancel event
│   │       ├── ping.rs      # Ping/heartbeat
│   │       └── shutdown.rs  # Shutdown event
│   └── handlers/    # HTTP API endpoints
│       ├── mod.rs
│       ├── chat.rs      # Chat CRUD endpoints
│       ├── message.rs   # Message endpoints
│       └── providers.rs # Provider listing
│
├── models/          # Shared data structures
│   ├── mod.rs
│   ├── users.rs
│   ├── workspaces.rs
│   ├── roles.rs
│   ├── files.rs
│   ├── chat.rs
│   └── requests.rs
│
├── queries/         # Shared database operations
│   ├── mod.rs
│   ├── users.rs
│   ├── workspaces.rs
│   ├── files.rs
│   ├── chat.rs
│   └── ai_models.rs
│
├── services/        # Shared business logic
│   ├── mod.rs
│   ├── users.rs
│   ├── workspaces.rs
│   ├── files.rs
│   └── storage.rs
│
├── handlers/        # Shared HTTP handlers
│   ├── mod.rs
│   ├── auth.rs
│   ├── workspaces.rs
│   ├── members.rs
│   └── files.rs
│
├── tools/           # Core tools (layered architecture)
│   └── ... (see TOOLS_LAYERED_ARCHITECTURE.md)
│
├── fs/              # File system core module
│   └── ... (layered architecture)
│
├── middleware/      # HTTP middleware
├── providers/       # AI provider integrations
├── cache/           # Caching layer
├── utils/           # Utility functions
├── parsers/         # File format parsers
└── workers/         # Background tasks
```

## Layered Architecture Pattern

The codebase follows a consistent layered architecture pattern across all modules:

### Layer Dependencies

```
┌─────────────────────────────────────────────────────────┐
│                     Handlers                             │
│  (HTTP API endpoints - thin orchestration layer)         │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                     Services                             │
│  (Business logic, validation, authorization)             │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                     Queries                              │
│  (Type-safe database operations, CRUD)                   │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                     Models                               │
│  (Data structures, validation rules, types)              │
└─────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

1. **Models Layer** (`models/`)
   - Data structures with Serde serialization
   - Validation rules and constraints
   - Database entity mappings
   - Request/response models for APIs

2. **Queries Layer** (`queries/`)
   - Type-safe database operations (SQLx)
   - CRUD operations for all entities
   - Transaction management
   - Raw SQL queries with parameter binding

3. **Services Layer** (`services/`)
   - Business logic and validation
   - Transaction coordination
   - Permission enforcement
   - Error handling and conversion
   - Cross-module orchestration

4. **Handlers Layer** (`handlers/`)
   - Thin orchestration: extract → validate → call service → return
   - HTTP request/response handling
   - 3-5 lines maximum per handler
   - NO business logic

## Agent Module

The `agent` module manages AI personas and agent session lifecycle.

### Module Organization

```
src/agent/
├── mod.rs           # Module root with get_persona() function
├── core/            # Agent personas (system prompts)
│   ├── assistant.rs # Default assistant
│   ├── planner.rs   # Strategic planning
│   └── builder.rs   # Plan execution
├── models/          # AgentSession, AgentType, SessionStatus
├── queries/         # Database operations
├── services/        # Session lifecycle management
└── handlers/        # REST API endpoints
```

### Key Exports

```rust
// From agent module root (src/agent/mod.rs)
pub use models::{
    AgentSession, AgentType, SessionStatus, NewAgentSession, UpdateAgentSession,
    AgentSessionInfo, AgentSessionsListResponse, PauseSessionRequest,
    ResumeSessionRequest, SessionActionResponse, SessionHeartbeat,
};
pub use services::{
    get_or_create_session, get_session, pause_session,
    resume_session, cancel_session, cleanup_stale_sessions,
    create_session, update_session_status, update_session_task,
    update_session_metadata, update_heartbeat, delete_session,
    list_workspace_sessions, list_user_sessions,
};
```

### Getting Agent Personas

The `get_persona()` function is the central registry for agent personas:

```rust
use buildscale::agent::get_persona;

// Get planner persona
let prompt = get_persona(Some("planner"), None, None);

// Get builder persona with plan content
let prompt = get_persona(Some("builder"), None, Some("Plan content here"));

// Auto-select based on mode
let prompt = get_persona(None, Some("plan"), None);

// Default to assistant
let prompt = get_persona(None, None, None);
```

**Function Signature:**
```rust
pub fn get_persona(
    role: Option<&str>,      // "planner", "builder", or None
    mode: Option<&str>,      // "plan", "build", or None
    plan_content: Option<&str> // Required for builder persona
) -> String
```

**Priority:** explicit role > mode-based selection > default assistant

## Chat Module

The `chat` module manages AI conversations with a complex service subsystem.

### Module Organization

```
src/chat/
├── mod.rs           # Module root with re-exports
├── models/          # ChatMessage, NewChatMessage, ChatAttachment
├── queries/         # Message CRUD operations
├── services/        # Complex subsystem
│   ├── mod.rs       # ChatService, BuiltContext, constants
│   ├── context.rs   # AttachmentManager, HistoryManager
│   ├── sync.rs      # YAML frontmatter sync
│   ├── registry.rs  # Session registry
│   ├── actor/       # Actor-based session management
│   ├── engine/      # Rig AI engine integration
│   ├── state_machine/ # State machine pattern
│   ├── states/      # State implementations
│   └── events/      # Event processors
└── handlers/        # REST API endpoints
```

### Key Exports

```rust
// From chat module root (src/chat/mod.rs)
pub use models::{ChatMessage, NewChatMessage, ChatAttachment};
pub use services::{ChatService, RigService, BuiltContext, ChatFrontmatter};
pub use services::actor::ChatActor;
pub use handlers::{create_chat, get_chat, post_chat_message, list_chats, get_providers};
```

### Service Subsystems

The `chat/services/` directory contains several sophisticated subsystems:

#### 1. Core Services (`services/mod.rs`, `context.rs`, `sync.rs`)
- **ChatService**: Main service for chat operations
- **BuiltContext**: Structured context for AI sessions (persona + history + attachments)
- **AttachmentManager**: Priority-based attachment pruning
- **HistoryManager**: Conversation history with token estimation
- **ChatFrontmatter**: YAML frontmatter serialization

#### 2. Actor System (`services/actor/`)
Manages concurrent chat sessions with SSE streaming:
- **ChatActor**: Actor-based session lifecycle
- **ProcessorContext**: Context for processing interactions
- **Streaming**: SSE event streaming to clients

#### 3. Engine (`services/engine/`)
Rig AI framework integration:
- **RigService**: Agent creation and configuration
- **Tools**: Thin adapters exposing workspace tools to AI

#### 4. State Machine (`services/state_machine/`, `services/states/`, `services/events/`)
Manages chat session states:
- **States**: Idle, Running, Paused, Completed, Cancelled, Error, Shutdown
- **Events**: Process, Pause, Cancel, Ping, Shutdown
- **Transitions**: Valid state transitions with side effects

### State Machine Architecture

```
┌─────────┐    process    ┌─────────┐
│  Idle   │──────────────>│ Running │
└─────────┘               └─────────┘
     ▲                    │    │
     │                    │    │
     │ pause              │    │ complete/cancel
     │                    ▼    ▼
     │               ┌─────────┐
     │               │ Paused  │
     │               └─────────┘
     │                    │
     │ resume             │ cancel
     └────────────────────┴──────────> [Completed/Cancelled/Error/Shutdown]
```

## Import Guide

### From Module Roots

```rust
// Agent module
use buildscale::agent::{
    get_persona,
    AgentSession, AgentType, SessionStatus,
};

// Chat module
use buildscale::chat::{
    ChatMessage, NewChatMessage, ChatAttachment,
    ChatService, RigService, BuiltContext,
};

// Chat actor (for session management)
use buildscale::chat::ChatActor;
```

### From Internal Layers

```rust
// Agent models
use crate::agent::models::{AgentSession, AgentType};

// Agent services
use crate::agent::services::{get_session, pause_session};

// Chat context management
use crate::chat::services::{
    AttachmentManager, HistoryManager, BuiltContext,
};

// Chat state machine
use crate::chat::services::state_machine::{ChatStateMachine, ChatState, ChatEvent};

// Chat actor
use crate::chat::services::actor::ChatActor;
```

### From lib.rs (Top-Level Exports)

```rust
// Agent exports
use buildscale::{get_persona, AgentSession, AgentType, SessionStatus};

// Common types
use buildscale::{Error, Result, DbConn, DbPool, Config};
```

## Backward Compatibility

The modules maintain backward compatibility through re-exports:

### Agent Module Re-exports
```rust
// src/agent/mod.rs
pub use models::{AgentSession, AgentType, SessionStatus, ...};
pub use services::{get_or_create_session, get_session, ...};
pub use handlers::{list_workspace_sessions_handler, ...};
```

### Chat Module Re-exports
```rust
// src/chat/mod.rs
pub use models::{ChatMessage, NewChatMessage, ChatAttachment};
pub use services::{ChatService, RigService, BuiltContext, ChatFrontmatter};
pub use services::actor::ChatActor;
```

## Entity Relationships

```
Users (1) <--> (N) Workspaces
   |                   |
   v                   v
   +--- Workspace Members ---> Roles (per workspace)
   |
   +--- User Sessions (authentication tokens)
   |
   +--- Agent Sessions (AI session tracking)
            |
            +---> Chat Files (conversations)
                      |
                      +---> Chat Messages (conversation history)
```

## Data Flow Architecture

### Request Processing Flow

```
HTTP Request --> Handler --> Service --> Query --> Database
                   |           |          |          |
                   v           v          v          v
              Extract     Validate    SQLx      PostgreSQL
              Params      Business    Query     Result
                          Logic       Exec
                   |           |          |          |
                   v           v          v          v
              Response <-- Format <-- Map <-- Entity
                          Result     to Model
```

### Chat Processing Flow

```
User Message --> Chat Handler --> ChatService.save_message()
                                      |
                                      v
                              DB Insert (chat_messages)
                                      |
                                      v
                              Disk Append (.chat file)
                                      |
                                      v
                              ChatActor.process()
                                      |
                                      v
                              Rig Engine (AI)
                                      |
                                      v
                              Tool Execution
                                      |
                                      v
                              SSE Stream Response
```

## Security Architecture

**Authentication**:
- Session-based authentication with random HMAC-signed tokens (256-bit randomness)
- Argon2 password hashing with unique salts
- Configurable session expiration (default: 30 days)

**Authorization**:
- Role-based access control with hardcoded permissions
- Workspace-level data isolation
- Invitation-based member onboarding

**Validation**:
- Input validation at model layer
- Email format verification
- Password strength requirements
- Token format validation (hex:hex format with HMAC signature)

## Testing Strategy

### Test Organization
- Unit tests for individual service functions
- Integration tests for complete workflows
- Database constraint testing
- Error scenario coverage

### Test Data Management
Uses `TestApp` and `TestDb` utilities:
- Automatic test database initialization
- Unique test prefixes for isolation
- Helper methods for creating test entities
- Automatic cleanup on test completion

## Development Guidelines

### Handler vs Service Responsibility

**Handler Layer (Thin)**:
- Pure orchestration: extract -> validate -> call service -> return response
- 3-5 lines maximum
- NO business logic

**Service Layer (Thick)**:
- ALL business logic
- ALL authorization logic
- Reusable across HTTP, CLI, tests

### Code Quality Guidelines

1. **Explicit Match Statements**: Always prefer explicit match cases over catch-all patterns
2. **UTF-8 Safe String Slicing**: Never use byte indexing on strings
3. **Flexible Deserializers**: Use flexible deserializers for all numeric/boolean tool parameters

For detailed guidelines, see the main CLAUDE.md file.

## Related Documentation

- [Tools Layered Architecture](./TOOLS_LAYERED_ARCHITECTURE.md) - Core tools module structure
- [Agent State Machine](./AGENT_STATE_MACHINE.md) - Detailed state machine documentation
- [Rig Integration](./RIG_INTEGRATION.md) - AI framework integration
- [Events System](./EVENTS_SYSTEM.md) - Event handling patterns
- [Authentication](./AUTHENTICATION.md) - Auth system details
