# File System Layered Architecture

This document describes the layered architecture for the file system in BuildScale.ai.

## Architecture Overview

```
Layer 4: workflow/pipeline  (future - orchestrates everything)
Layer 3: agents             (uses tools)        <- src/agents/
Layer 2: tools              (AI interface)      <- src/tools/
Layer 2: handlers           (REST interface)    <- src/handlers/
Layer 1: fs                 (core)              <- src/fs/
```

## Layer Descriptions

### Layer 1: fs (Core File System)

**Location**: `src/fs/`

The foundation layer. Contains all file system domain logic with NO dependencies on HTTP or AI interfaces.

**Modules**:
- `models.rs` - File data structures (File, NewFile, FileType, etc.)
- `queries.rs` - Database CRUD operations for files
- `services.rs` - Business logic (create, update, delete, version management)
- `storage.rs` - Disk operations (FileStorageService)
- `parsers/` - Content extraction (links, tags)
- `workers/` - Background indexing
- `utils/` - Document metadata, frontmatter utilities

**Dependencies**: Only database, disk storage, and standard library.

### Layer 2: Interfaces

Two parallel interfaces consume `fs`:

#### REST Handlers (`src/handlers/files.rs`)
- HTTP JSON API
- Axum extraction and routing
- HTTP error codes and status
- Calls `fs::services` and `fs::queries`

#### AI Tools (`src/tools/`)
- Tool interface for AI agents
- JSON schema definitions
- Input validation and normalization
- Plan mode restrictions
- Signals `fs::workers` for indexing

### Layer 3: Agents

**Location**: `src/agents/`

AI agents that use tools to accomplish tasks. Agents don't interact with `fs` directly - they go through tools.

### Layer 4: Workflow/Pipeline (Future)

Orchestrates multiple agents, workers, and tools for complex workflows.

## Data Flow Examples

### REST API: Create File
```
POST /api/v1/workspaces/{id}/files
    |
handlers::files::create_file()
    |
fs::services::create_file()
    |
fs::queries::create_file() + fs::storage::write_file()
    |
Database + Disk
```

### AI Tool: Write File
```
AI Agent calls write_tool
    |
tools::write::execute()
    |
fs::services::create_file() or fs::services::update_file()
    |
fs::queries + fs::storage
    |
Signal: tag_indexer, link_indexer
    |
Database + Disk
```

## Design Principles

1. **Single Source of Truth**: `fs` is the only place for file system logic
2. **Interface Independence**: REST and AI can evolve independently
3. **Clear Dependencies**: Each layer only depends on layers below it
4. **Testability**: `fs` can be tested without HTTP or AI concerns
5. **Future-Ready**: Easy to add new interfaces (CLI, gRPC) or layers (workflow)

## Module Structure

```
src/fs/
├── mod.rs              # Public API exports
├── models.rs           # File data structures (File, NewFile, FileType, etc.)
├── queries.rs          # File database operations
├── services.rs         # File business logic
├── storage.rs          # FileStorageService (disk operations)
├── parsers/
│   ├── mod.rs
│   ├── links.rs        # extract_links()
│   └── tags.rs         # extract_tags()
├── workers/
│   ├── mod.rs
│   ├── link_indexer.rs # Background link indexing
│   └── tag_indexer.rs  # Background tag indexing
└── utils/
    ├── mod.rs
    ├── document_metadata.rs
    ├── frontmatter.rs
    └── yaml_frontmatter.rs
```

## Backward Compatibility

For backward compatibility with existing code, the original module paths re-export from `fs`:

- `src/models/files` -> re-exports from `crate::fs::models`
- `src/queries/files` -> re-exports from `crate::fs::queries`
- `src/services/files` -> re-exports from `crate::fs::services`
- `src/services/storage` -> re-exports from `crate::fs::storage`
- `src/parsers/` -> re-exports from `crate::fs::parsers`
- `src/workers/` -> re-exports link/tag indexers from `crate::fs::workers`
- `src/utils/` -> re-exports document utils from `crate::fs::utils`

This allows existing imports like `crate::models::files::File` to continue working.

## Migration Guide

When adding new file system functionality:

1. **Core logic** -> Add to `src/fs/`
2. **REST API** -> Add handler in `src/handlers/files.rs`, call `fs`
3. **AI Tool** -> Add tool in `src/tools/`, call `fs`
4. **Both** -> Add core logic once in `fs`, both interfaces use it

### Example: Adding a new file operation

```rust
// 1. Add core logic in src/fs/services.rs
pub async fn copy_file(
    conn: &mut DbConn,
    storage: &FileStorageService,
    source_id: Uuid,
    target_path: &str,
) -> Result<File> {
    // Implementation here
}

// 2. REST handler in src/handlers/files.rs calls fs
pub async fn copy_file(
    State(state): State<AppState>,
    // ...
) -> Result<Json<FileWithContent>> {
    let file = fs::services::copy_file(&mut conn, &storage, source_id, target_path).await?;
    Ok(Json(file.into()))
}

// 3. AI tool in src/tools/copy.rs calls fs
impl Tool for CopyTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        let file = fs::services::copy_file(&mut conn, &storage, source_id, target_path).await?;
        // Signal indexers if needed
        Ok(json!(file))
    }
}
```

## Benefits

- **Maintainability**: File system logic in one place
- **Consistency**: Same business rules for REST and AI
- **Testing**: Core logic can be unit tested without HTTP layer
- **Extensibility**: Easy to add new interfaces (CLI, WebSocket, etc.)
- **Performance**: No unnecessary abstraction overhead
