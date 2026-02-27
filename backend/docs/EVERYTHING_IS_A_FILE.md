← [Back to Index](./README.md) | **The Vision**: [Files Are All You Need](./FILES_ARE_ALL_YOU_NEED.md)

# Everything is a File: Database Architecture

This document details the "Everything is a file" philosophy and database implementation for BuildScale.ai. This architecture unifies documents, folders, chat sessions, and canvases into a single, cohesive system that supports both hierarchical organization (folders) and network organization (Obsidian-style wikilinks and tags).

## Core Philosophy

In BuildScale.ai, **Identity** and **Content** are unified in a simple, elegant model.

1. **Identity + Content (`files`)**: The permanent anchor with inline content tracking. It has an ID, a human name, a location (`parent_id`), and a SHA-256 `hash` of its current content.
2. **History (`versions` array)**: Every "Save" appends the previous hash to the `versions` array. We can restore any version by looking up the hash in the archive.
3. **Storage (Hybrid)**: The actual content lives on **Disk** (for speed and tooling compatibility), while the **Database** acts as the index and metadata registry.

**Key Simplifications** (Obsidian-Style):
- **Links** (`[[wikilinks]]`) and **Tags** (`#hashtags`) are parsed from content
- Background indexers populate `links` and `tags` tables for fast lookups
- No AI semantic search or vector embeddings
- Version history tracked via simple `TEXT[]` array

## Storage Layout

The system manages workspace directories within `/app/storage/workspaces/` (configurable via `STORAGE_BASE_PATH`):

**Directory Structure:**
```
/app/storage/workspaces/{workspace_id}/
├── latest/       # Current files (Source of Truth)
├── archive/      # Hash-based backup copies
└── trash/        # Soft-deleted files
```

### 1. The Latest (`latest/`)
*   **Structure**: Hierarchical storage - files stored at full logical path
*   **Example**: `./storage/workspaces/{workspace_id}/latest/projects/backend/src/main.rs`
*   **Usage**: All `read`, `ls`, `grep`, and AI tool operations hit this directory.
*   **State**: Always contains the current version of each file.
*   **Note**: Folders are created as actual directories on disk

### 2. The Archive (`archive/`)
*   **Structure**: Content-addressed storage with 2-level sharding.
*   **Example**: `./storage/workspaces/{workspace_id}/archive/e3/b0/e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
*   **Usage**: History lookup, version restoration.
*   **State**: Immutable blobs addressed by content hash.
*   **Key**: The `hash` column points to these files. Same content = same hash (deduplication).

### 3. The Trash (`trash/`)
*   **Structure**: Hierarchical list of deleted files preserving folder structure.
*   **Example**: `./storage/workspaces/{workspace_id}/trash/projects/backend/main.rs`
*   **Usage**: Soft delete recovery.

## Schema Overview

### The Registry: `files`

The `files` table is the **single** registry for all objects in the system. No separate version, link, or tag tables.

| Column | Type | Description |
|---|---|---|
| `id` | UUID (v7) | Unique identifier. |
| `workspace_id` | UUID | Tenant isolation. |
| `parent_id` | UUID | **The Folder Structure.** Points to the parent folder file. `NULL` = Root. |
| `file_type` | TEXT | `document`, `folder`, `canvas`, `chat`, `whiteboard`, `agent`, `skill`, `plan`, `memory`. |
| `name` | TEXT | **Display Name.** Supports spaces, emojis, mixed case (e.g., "My Plan ✨"). |
| `path` | TEXT | **Materialized Path.** Absolute path for fast tree queries (e.g., "/my-plan/doc"). Unique per workspace. |
| `hash` | TEXT | **Content Hash.** SHA-256 of current content. NULL for folders. |
| `versions` | TEXT[] | **History.** Array of historical hashes (newest first). Enables version restore. |
| `deleted_at` | TIMESTAMPTZ | **Trash Bin.** If not NULL, the file is in the trash. |
| `created_at` | TIMESTAMPTZ | Creation timestamp. |
| `updated_at` | TIMESTAMPTZ | Last update timestamp. |

**Constraints:**
- `path` is unique per workspace for active files (`WHERE deleted_at IS NULL`)
- `file_type` must be one of the valid types (check constraint)

**Folder Logic:**
*   A **Folder** is just a row in `files` with `file_type = 'folder'`.
*   To put a file in a folder, set its `parent_id` to the folder's `id`.
*   Folders have `hash = NULL` (no content).

**Trash Logic:**
*   **Soft Delete**: When a file is deleted, `deleted_at` is set to the current timestamp.
*   **Unique Constraints**: A deleted file releases its claim on `path`. You can create a new file with the same path as a deleted one.
*   **Purge (Hard Delete)**: An irreversible operation that removes the file from the database.

## Content-Addressed Versioning

### Write Flow
```
1. Calculate hash = SHA-256(new_content)
2. If file exists:
   a. Archive current content: copy latest/ to archive/{hash[:2]}/{hash[2:4]}/{hash}
   b. Append old hash to versions array
3. Write new content to latest/{path}
4. Update hash in database
```

### Restore Flow
```
1. Read content from archive/{hash[:2]}/{hash[2:4]}/{hash}
2. Write to latest/{path}
3. Move current hash to versions array
4. Set hash = restored_version_hash
```

### Benefits
- **Deduplication**: Same content = same hash = single storage blob
- **Integrity**: Hash mismatch detects external modifications
- **Simplicity**: No complex version table with joins

## Knowledge Graph (Obsidian-Style)

### Wikilinks
Links are extracted from content using the `[[wikilink]]` syntax:

```markdown
See [[Project Alpha]] for details.
See [[meetings/standup|Daily Standup]] for the schedule.
```

**Extraction**:
```rust
pub fn extract_links(content: &str) -> Vec<String> {
    // Matches [[link]] or [[path|display]]
    Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]")
        .captures_iter(content)
        .map(|c| c[1].to_string())
        .collect()
}
```

### Hashtags
Tags are extracted from content using the `#hashtag` syntax:

```markdown
This is #important for #project/alpha.
```

**Extraction**:
```rust
pub fn extract_tags(content: &str) -> Vec<String> {
    // Matches #tag (ignores headings like # Heading)
    Regex::new(r"#[\w/\-]+")
        .captures_iter(content)
        .map(|c| c[1].to_lowercase())
        .collect()
}
```

### Backlinks
To find files that link TO a file, use the indexed `links` table:

```sql
-- Fast O(1) backlink lookup using the links index
SELECT DISTINCT f.name
FROM files f
INNER JOIN links l ON l.source_file_id = f.id
WHERE l.workspace_id = $1
  AND l.target_name = 'target-file-name'
  AND f.deleted_at IS NULL;
```

**How it works**:
1. Background worker listens for file changes
2. Extracts wikilinks using `extract_links()`
3. Stores `(workspace_id, source_file_id, target_name)` in `links` table
4. Enables fast backlink lookups without file scanning

## Common Access Patterns

### A. Folder Navigation (Sidebar)
"Show me everything in the 'Marketing' folder."

```sql
SELECT * FROM files
WHERE parent_id = 'uuid-of-marketing-folder'
  AND deleted_at IS NULL
ORDER BY (file_type = 'folder') DESC, name ASC;
```

### B. Latest Content (Opening a File)
"Get the current content for file X."

1. Query DB for metadata/hash.
2. Read file from `./storage/workspaces/{workspace_id}/latest/<path>`.

### C. Hierarchy Lookup (Materialized Path)
"Get all files in the 'Projects' folder and all its subfolders."

```sql
-- Fast O(log N) lookup without recursion
SELECT * FROM files
WHERE workspace_id = 'current-workspace'
  AND (path = '/projects' OR path LIKE '/projects/%')
  AND deleted_at IS NULL
ORDER BY path ASC;
```

### D. Version History
"Get all versions of file X."

```sql
SELECT id, name, hash, versions
FROM files
WHERE id = 'file-uuid';
-- versions array contains all historical hashes
```

### E. Find Files by Tag (Indexed)
"Find all files tagged #important."

```sql
-- Fast lookup using the tags index
SELECT f.*
FROM files f
INNER JOIN tags t ON t.file_id = f.id
WHERE t.workspace_id = $1
  AND t.tag = 'important'
  AND f.deleted_at IS NULL;
```

**How it works**:
1. Background worker listens for file changes
2. Extracts hashtags using `extract_tags()`
3. Stores `(workspace_id, file_id, tag)` in `tags` table
4. Enables fast tag-based file lookups without scanning content
