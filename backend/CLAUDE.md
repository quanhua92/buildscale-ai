# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

```bash
# Build
cargo build

# Run tests
cargo test

# Run specific test
cargo test test_user_registration_success

# Run examples
cargo run --example 01_hello

# Database migrations
sqlx migrate run
sqlx migrate info
```

## Architecture Overview

Multi-tenant workspace-based RBAC system with layered architecture:

```
src/
├── users/           # User management (layered)
├── workspaces/      # Workspace management (layered)
├── auth/            # Authentication (layered)
├── ai/              # AI providers and models (layered)
├── chat/            # Chat and agent services (layered)
├── tools/           # Core tools (layered)
│   ├── file/        # File system tools (14 tools)
│   ├── memory/      # Memory tools (5 tools)
│   ├── plan/        # Plan mode tools (6 tools)
│   └── web/         # Web tools (2 tools)
├── fs/              # File system core (layered)
├── workers/         # Background workers (layered)
└── middleware/      # HTTP middleware
```

### Layer Pattern

Each domain module follows the three-layer pattern:
- **Models**: Data structures, validation rules, type definitions
- **Queries**: Type-safe database operations
- **Services**: Business logic, validation, workflows
- **Handlers**: Thin HTTP orchestration (3-5 lines max)

## Key Design Patterns

### Handler vs Service

**Handler (Thin)**: Extract → Validate → Call Service → Return
```rust
pub async fn update_workspace(
    Extension(access): Extension<WorkspaceAccess>,
    DbConnection(mut conn): DbConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRequest>,
) -> Result<Json<Value>> {
    let workspace = workspaces::update_workspace(&mut conn, id, req).await?;
    Ok(Json(json!({ "workspace": workspace })))
}
```

**Service (Thick)**: ALL business logic, ALL authorization logic

### Tool Architecture

- **Core Tools (`src/tools/`)**: Thick layer with ALL execution logic
- **AI Adapters (`src/chat/services/`)**: Thin translation layer to Rig framework

### Flexible Deserializers (CRITICAL)

ALL tool parameters accepting numeric/boolean values MUST use flexible deserializers:

```rust
#[serde(default, deserialize_with = "deserialize_flexible_bool_option")]
pub recursive: Option<bool>,

#[serde(default, deserialize_with = "deserialize_flexible_usize_option")]
pub limit: Option<usize>,
```

## Code Quality Guidelines

### Explicit Match Statements

Always prefer explicit match cases over catch-all patterns:

```rust
// GOOD - Explicit with warning for unknown
match tool_name {
    "write" => { /* handle */ }
    "read" => { /* handle */ }
    unknown => {
        tracing::warn!("Unknown tool '{}'", unknown);
    }
}
```

### UTF-8 Safe String Slicing

Never use byte indexing on strings. Use character-aware methods:

```rust
use crate::utils::safe_preview;
let preview = safe_preview(&text, 50);
```

## Configuration

Environment variables use `BUILDSCALE__` prefix:

| Variable | Description |
|----------|-------------|
| `BUILDSCALE__DATABASE__*` | Database connection |
| `BUILDSCALE__JWT__SECRET` | JWT signing key (min 32 chars) |
| `BUILDSCALE__JWT__ACCESS_TOKEN_EXPIRATION_MINUTES` | JWT expiration (default: 15) |
| `BUILDSCALE__SESSIONS__EXPIRATION_HOURS` | Session expiration (default: 720) |

## Error Handling

```rust
pub enum Error {
    Sqlx(#[from] sqlx::Error),
    Validation(ValidationErrors),
    NotFound(String),
    Forbidden(String),
    Conflict(String),
    Authentication(String),
    InvalidToken(String),
    SessionExpired(String),
    Internal(String),
}
```

All errors include `error` message and `code` field for programmatic handling.

## Documentation

Comprehensive documentation is in `docs/`:

| Document | Content |
|----------|---------|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | System architecture |
| [API_REFERENCE.md](docs/API_REFERENCE.md) | REST, Tools, Services APIs |
| [AI_SYSTEM.md](docs/AI_SYSTEM.md) | AI agents, context, providers |
| [AUTHENTICATION.md](docs/AUTHENTICATION.md) | Auth, RBAC, invitations |
| [FILE_SYSTEM.md](docs/FILE_SYSTEM.md) | File system architecture |
| [PLAN_SYSTEM.md](docs/PLAN_SYSTEM.md) | Plan mode workflow |
| [CONFIGURATION.md](docs/CONFIGURATION.md) | Configuration reference |

## Development Workflow

1. **Code**: Models → Services → Queries → Handlers
2. **Test**: Unit tests, integration tests, edge cases
3. **Example**: Create practical examples in `/examples/`
4. **Document**: Update relevant docs

Final validation:
```bash
cargo test && cargo build --release
```
