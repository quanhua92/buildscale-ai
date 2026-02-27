# Workers Module Layered Architecture

## Overview

The workers module was reorganized from a flat structure to a layered structure, following the same pattern as `src/tools/` and `src/fs/` modules.

## Module Structure

```
src/workers/
├── mod.rs              # Worker registry, re-exports
├── auth/               # Authentication workers
│   ├── mod.rs
│   └── token_cleanup.rs
└── storage/            # Storage workers
    ├── mod.rs
    └── archive_cleanup.rs
```

## Design Principles

### 1. Categorization

Workers are grouped by functionality:
- **auth/**: Authentication token cleanup (`revoked_token_cleanup_worker`)
- **storage/**: Storage blob cleanup (`archive_cleanup_worker`)

### 2. Consistency

This structure follows the same pattern as `src/tools/` and `src/fs/`.

## Worker Descriptions

### auth/token_cleanup.rs
- **Purpose**: Background cleanup of expired revoked JWT refresh tokens
- **Frequency**: Runs every 5 minutes
- **Function**: `revoked_token_cleanup_worker()`

### storage/archive_cleanup.rs
- **Purpose**: Immediate deletion of orphaned archive blob files
- **Trigger**: Message-driven (listens for cleanup messages)
- **Function**: `archive_cleanup_worker()`

## Adding a New Worker

1. Choose the appropriate category (auth, storage, or create new)
2. Create the worker file in that subdirectory
3. Add to the subdirectory's mod.rs (both `mod` and `pub use`)
4. Add re-export to main workers/mod.rs

### Example: Adding a New Worker

```bash
# 1. Create the worker file
touch src/workers/storage/new_cleanup.rs

# 2. Add to storage/mod.rs
echo "mod new_cleanup;" >> src/workers/storage/mod.rs
echo "pub use new_cleanup::new_cleanup_worker;" >> src/workers/storage/mod.rs

# 3. Add re-export to main mod.rs
echo "pub use storage::new_cleanup_worker;" >> src/workers/mod.rs
```

## Related Documentation

- [Architecture](./ARCHITECTURE.md) - System architecture overview
- [Tools Layered Architecture](./TOOLS_LAYERED_ARCHITECTURE.md) - Tools module structure
