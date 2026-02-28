# File System Architecture

BuildScale.ai transforms a standard file system into a **Distributed Operating System** for AI agents using the "Everything is a File" philosophy.

## Table of Contents

- [Core Philosophy](#core-philosophy)
- [Storage Architecture](#storage-architecture)
- [Database Schema](#database-schema)
- [File Types](#file-types)
- [Knowledge Graph](#knowledge-graph)
- [Memory Tools](#memory-tools)
- [Common Access Patterns](#common-access-patterns)

---

## Core Philosophy

In BuildScale, **Everything is a File**. Every workspace is a self-contained "Operating System" where AI interacts through a standardized folder taxonomy and unified toolset.

### The Standardized Taxonomy

Every workspace shares a consistent root structure:

| Path | Purpose |
|------|---------|
| `/` (Root) | Container for the entire logical volume |
| `/system/skills/<skill_name>/SKILL.md` | The "Toolbox" - capability definitions |
| `/system/agents/<agent_name>/AGENT.md` | The "Staff" - agent persona definitions |
| `/chats/chat-{id}.chat` | The "Memory" - conversation logs with YAML frontmatter |
| `/data/` | The "Knowledge" - ingested raw documents (PDFs, Videos, CSVs) |
| `/users/<user_id>/` | The "Home Directory" - user-specific workspace state |
| `/projects/<project_name>/` | The "Project" - user's codebase and working files |
| `/memories/` | Global memories shared across workspace |
| `/plans/` | Plan files for structured workflows |

### Key Concepts

1. **Identity + Content (`files`)**: The permanent anchor with inline content tracking
   - Has an ID, a human name, a location (`parent_id`)
   - SHA-256 `hash` of its current content

2. **History (`versions` array)**: Every "Save" appends the previous hash to the `versions` array

3. **Storage (Hybrid)**: Content lives on **Disk** (for speed and tooling compatibility), while the **Database** acts as the index and metadata registry

---

## Storage Architecture

The system manages workspace directories within `/app/storage/workspaces/` (configurable via `STORAGE_BASE_PATH`).

### Directory Structure

```
/app/storage/workspaces/{workspace_id}/
├── latest/       # Current files (Source of Truth)
├── archive/      # Hash-based backup copies
└── trash/        # Soft-deleted files
```

### 1. The Latest (`latest/`)

- **Structure**: Hierarchical storage - files stored at full logical path
- **Example**: `./storage/workspaces/{workspace_id}/latest/projects/backend/src/main.rs`
- **Usage**: All `read`, `ls`, `grep`, and AI tool operations hit this directory
- **State**: Always contains the current version of each file
- **Note**: Folders are created as actual directories on disk

### 2. The Archive (`archive/`)

- **Structure**: Content-addressed storage with 2-level sharding
- **Example**: `./storage/workspaces/{workspace_id}/archive/e3/b0/e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- **Usage**: History lookup, version restoration
- **State**: Immutable blobs addressed by content hash
- **Key**: Same content = same hash (deduplication)

### 3. The Trash (`trash/`)

- **Structure**: Hierarchical list of deleted files preserving folder structure
- **Example**: `./storage/workspaces/{workspace_id}/trash/projects/backend/main.rs`
- **Usage**: Soft delete recovery

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

---

## Database Schema

### The Registry: `files`

The `files` table is the **single** registry for all objects in the system.

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID (v7) | Unique identifier |
| `workspace_id` | UUID | Tenant isolation |
| `parent_id` | UUID | Points to parent folder. `NULL` = Root |
| `file_type` | TEXT | `document`, `folder`, `canvas`, `chat`, `whiteboard`, `agent`, `skill`, `plan`, `memory` |
| `name` | TEXT | Display Name (supports spaces, emojis, mixed case) |
| `path` | TEXT | Materialized Path for fast tree queries. Unique per workspace |
| `hash` | TEXT | SHA-256 of current content. NULL for folders |
| `versions` | TEXT[] | Array of historical hashes (newest first) |
| `deleted_at` | TIMESTAMPTZ | If not NULL, the file is in the trash |
| `created_at` | TIMESTAMPTZ | Creation timestamp |
| `updated_at` | TIMESTAMPTZ | Last update timestamp |

### Constraints

- `path` is unique per workspace for active files (`WHERE deleted_at IS NULL`)
- `file_type` must be one of the valid types (check constraint)

### Folder Logic

- A **Folder** is just a row in `files` with `file_type = 'folder'`
- To put a file in a folder, set its `parent_id` to the folder's `id`
- Folders have `hash = NULL` (no content)

### Trash Logic

- **Soft Delete**: When a file is deleted, `deleted_at` is set to the current timestamp
- **Unique Constraints**: A deleted file releases its claim on `path`
- **Purge (Hard Delete)**: An irreversible operation that removes the file from the database

---

## File Types

| Type | Description | Content Location |
|------|-------------|------------------|
| `document` | Regular files (markdown, code, etc.) | `latest/{path}` |
| `folder` | Directory container | No content |
| `chat` | Conversation with YAML frontmatter | `latest/chats/chat-{id}.chat` |
| `canvas` | Visual canvas data | `latest/{path}` |
| `whiteboard` | Drawing/whiteboard data | `latest/{path}` |
| `agent` | Agent persona definition | `latest/system/agents/{name}/AGENT.md` |
| `skill` | Skill/capability definition | `latest/system/skills/{name}/SKILL.md` |
| `plan` | Structured plan file | `latest/plans/{name}.md` |
| `memory` | Persistent memory file | `latest/users/{user_id}/memories/` or `latest/memories/` |

---

## Knowledge Graph

BuildScale uses Obsidian-style content parsing for links and tags.

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

---

## Memory Tools

The Memory Management System provides persistent, long-term memory capabilities for AI agents. Memories are stored as Markdown files with YAML frontmatter.

### Memory Scopes

| Scope | Path Pattern | Visibility |
|-------|--------------|------------|
| `user` | `/users/{user_id}/memories/{category}/{key}.md` | Private to the user |
| `global` | `/memories/{category}/{key}.md` | Shared across workspace |

### Memory File Format

```markdown
---
title: "Meeting Notes: Q4 Planning"
tags: ["meeting", "planning", "q4"]
category: "work"
created_at: "2025-01-15T10:30:00Z"
updated_at: "2025-01-15T10:30:00Z"
scope: "user"
---

# Meeting Notes: Q4 Planning

Content here...
```

### Memory Tools

| Tool | Description |
|------|-------------|
| `memory_set` | Creates or updates a memory with metadata |
| `memory_get` | Retrieves a specific memory by scope, category, and key |
| `memory_search` | Searches across all memories with filtering |
| `memory_list` | Lists categories, tags, or memories efficiently |
| `memory_delete` | Deletes a specific memory (soft delete) |

### Recommended Categories

| Category | Purpose | Example Keys |
|----------|---------|--------------|
| `preferences` | User preferences | `coding-style`, `editor-config`, `language` |
| `project` | Project-specific context | `api-endpoints`, `tech-stack`, `folder-structure` |
| `decisions` | Architecture/design decisions | `auth-strategy`, `database-choice`, `api-design` |
| `context` | General work context | `current-focus`, `team-info`, `deadlines` |
| `corrections` | User corrections to remember | `no-console-log`, `prefer-async` |
| `patterns` | Discovered patterns | `error-handling`, `naming-convention` |
| `references` | Quick reference guides | `git-commands`, `docker-cheatsheet` |

---

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

1. Query DB for metadata/hash
2. Read file from `./storage/workspaces/{workspace_id}/latest/<path>`

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

### E. Find Files by Tag

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

---

## The Semantic Toolset

Agents use a standardized, semantic developer toolset to interact with the file system.

### Discovery (`ls`, `glob`)

- **Purpose**: Exploring the environment
- **Example**: `ls /system/skills` to see what tools are available

### Ingestion (`read`)

- **Purpose**: Absorbing context into the agent's window
- **Mechanism**: Reads the actual file from disk (supporting truncation and markdown conversion)

### Recall (`grep`)

- **Purpose**: Finding specific information across the entire logical volume
- **Mechanism**: High-speed text search across the workspace

### Action (`edit`, `write`)

- **`edit`**: Atomic modifications with precise "search & replace" blocks
- **`write`**: Creating new permanent state (plans, artifacts, code)

---

## Related Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System architecture overview
- [API_REFERENCE.md](./API_REFERENCE.md) - Complete API reference
- [AI_SYSTEM.md](./AI_SYSTEM.md) - AI agent system architecture
