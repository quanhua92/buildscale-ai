← [Back to Index](./README.md) | **The Vision**: [Files Are All You Need](./FILES_ARE_ALL_YOU_NEED.md)

# Everything is a File: Database Architecture

This document details the "Everything is a file" philosophy and database implementation for BuildScale.ai. This architecture unifies documents, folders, chat sessions, and canvases into a single, cohesive system that supports both hierarchical organization (folders) and network organization (knowledge graph).

## Core Philosophy

In BuildScale.ai, **Identity** is separated from **Content**.

1.  **Identity (`files`)**: The permanent anchor. It has an ID, a human name (`name`), a machine identifier (`slug`), a location (`parent_id`), and an owner. It doesn't change when you edit the document content.
2.  **Content (`file_versions`)**: The immutable history. Every "Save" creates a new version. We never update existing content; we only append new versions.
3.  **Storage (Hybrid)**: The actual content lives on **Disk** (for speed and tooling compatibility), while the **Database** acts as the index and metadata registry.

**Default Type & Validation**:
*   The system defaults to `file_type = 'document'` if not specified.
*   **Documents** and **Chats** can contain raw text content or JSON objects.
*   **Canvas** and **Whiteboard** types use JSON structures for complex data.
*   **Folders** are identity-only nodes and do not typically hold text content.

## Storage Layout

The system manages workspace directories within `/app/storage/workspaces/` (configurable via `STORAGE_BASE_PATH`):

**Directory Structure:**
```
/app/storage/workspaces/{workspace_id}/
├── latest/       # Current files (Source of Truth)
├── archive/      # All file versions (Content-Addressable Store)
└── trash/        # Soft-deleted files
```

### 1. The Latest (`latest/`)
*   **Structure**: Hierarchical storage - files stored at full logical path
*   **Example**: `./storage/workspaces/{workspace_id}/latest/projects/backend/src/main.rs`
*   **Usage**: All `read`, `ls`, `grep`, and AI tool operations hit this directory.
*   **State**: Always contains the `latest_version` of file.
*   **Note**: Folders are created as actual directories on disk

### 2. The Archive (`archive/`)
*   **Structure**: Version-unique storage with 2-level sharding.
*   **Example**: `./storage/workspaces/{workspace_id}/archive/e3/b0/e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
*   **Usage**: History lookup, irreversible restoration.
*   **State**: Immutable blobs.
*   **Key**: The `hash` column in `file_versions` points to these files. Hashes are salted with `version_id` to ensure every version has a unique physical blob, simplifying reliable storage reclamation.

### 3. The Trash (`trash/`)
*   **Structure**: Hierarchical list of deleted files preserving folder structure.
*   **Example**: `./storage/workspaces/{workspace_id}/trash/projects/backend/main.rs`
*   **Usage**: Soft delete recovery.

## Schema Overview

### 1. The Registry: `files`

The `files` table is the central registry for all objects in the system.

| Column | Type | Description |
|---|---|---|
| `id` | UUID (v7) | Unique identifier. |
| `workspace_id` | UUID | Tenant isolation. |
| `parent_id` | UUID | **The Folder Structure.** Points to the parent folder file. `NULL` = Root. |
| `author_id` | UUID | **Creator.** Nullable to support user deletion (`ON DELETE SET NULL`). |
| `file_type` | TEXT | `document`, `folder`, `canvas`, `chat`, `whiteboard`. |
| `status` | TEXT | `pending`, `uploading`, `waiting`, `processing`, `ready`, `failed`. |
| `name` | TEXT | **Display Name.** Supports spaces, emojis, mixed case (e.g., "My Plan ✨"). |
| `slug` | TEXT | **URL-safe Name.** Lowercase, hyphens (e.g., "my-plan"). Unique per folder. |
| `path` | TEXT | **Materialized Path.** Absolute path for fast tree queries (e.g., "/my-plan/doc"). Unique per workspace. |
| `is_virtual` | BOOLEAN | **Optimization Flag.** If true, it implies the file might be constructed dynamically or has special handling (like Chats), though in the hybrid model most files are physical. |
| `is_remote` | BOOLEAN | **Storage Flag.** If true, content **must** be fetched from S3/Object Storage; otherwise it is local disk. |
| `permission` | INT | **Unix-style Mode.** Access control (e.g. 600 for private, 755 for shared). Defaults to 600. |
| `latest_version_id` | UUID | **Cache.** Points to the most recent version in `file_versions`. |
| `deleted_at` | TIMESTAMPTZ | **Trash Bin.** If not NULL, the file is in the trash. |
| `created_at` | TIMESTAMPTZ | Creation timestamp. |
| `updated_at` | TIMESTAMPTZ | Last update timestamp (for metadata). |

**Folder Logic:**
*   A **Folder** is just a row in `files` with `file_type = 'folder'`.
*   To put a file in a folder, set its `parent_id` to the folder's `id`.

**Trash Logic:**
*   **Soft Delete**: When a file is deleted, `deleted_at` is set to the current timestamp.
*   **Unique Constraints**: A deleted file releases its claim on the `slug` and `path`. You can create a new file with the same path as a deleted one.
*   **Purge (Hard Delete)**: An irreversible operation that removes the file registry entry from the database.
*   **Cascade Cleanup**: Deleting a file automatically triggers an `ON DELETE CASCADE` in the database, removing its Versions, Tags, Links, and Chat Messages.
*   **Automated Archive Cleanup**: A background worker periodically identifies orphaned blobs in the `archive/` folder (hashes no longer referenced by any file version) and physically deletes them from disk using a `SKIP LOCKED` concurrency pattern.

### 2. The Content: `file_versions`

Stores the history and metadata of file changes.

**Architectural Change**:
*   Content is stored exclusively on disk in the **Archive**.
*   The `hash` column points to the file content at `./archive/{hash}`.
*   Database stores only metadata (no content).
*   **Isolation**: Every version has a unique hash salted with its `version_id`. This ensures that deleting one version or file never affects another, enabling safe and immediate physical cleanup.

| Column | Type | Description |
|---|---|---|
| `id` | UUID | Unique version identifier. |
| `file_id` | UUID | Link to the Identity. |
| `workspace_id` | UUID | **Tenant isolation.** Denormalized for performance. |
| `author_id` | UUID | Who created this specific version. Supports user deletion. |
| `app_data` | JSONB | Machine metadata (storage type, size, preview, AI tags, etc.). |
| `hash` | TEXT | **The Key**. Unique SHA-256 hash of content salted with `version_id`. |
| `branch` | TEXT | Default `main`. Supports A/B variants. |
| `created_at` | TIMESTAMPTZ | Version creation timestamp. |
| `updated_at` | TIMESTAMPTZ | Metadata update timestamp. |

### 3. The Knowledge Graph: `file_links`

Represents connections *between* files, independent of folder structure.

| Column | Type | Description |
|---|---|---|
| `source_file_id` | UUID | Where the link originates. |
| `target_file_id` | UUID | What is being linked to. |
| `workspace_id` | UUID | **Tenant isolation.** |
| `created_at` | TIMESTAMPTZ | Link creation timestamp. |
| `updated_at` | TIMESTAMPTZ | Metadata update timestamp. |

### 4. The Taxonomy: `file_tags`

High-performance categorization.

| Column | Type | Description |
|---|---|---|
| `file_id` | UUID | The file. |
| `workspace_id` | UUID | **Tenant isolation.** |
| `tag` | TEXT | Normalized tag string (e.g., "marketing"). |
| `created_at` | TIMESTAMPTZ | Tag assignment timestamp. |
| `updated_at` | TIMESTAMPTZ | Metadata update timestamp. |

### 5. Semantic Memory: `file_chunks` & `file_version_chunks`

Optimized RAG storage with deduplication and model upgrade support.

**`file_chunks` (The Pool):**
Stores unique text snippets and embeddings.
| Column | Type | Description |
|---|---|---|
| `id` | UUID | Chunk ID. |
| `workspace_id` | UUID | Tenant isolation. |
| `chunk_hash` | TEXT | SHA-256 of content. Unique per workspace. |
| `chunk_content`| TEXT | The text snippet. |
| `embedding` | vector | AI-generated embedding (e.g., 1536d). Updated on conflict. |

**`file_version_chunks` (The Map):**
Links versions to their chunks.
| Column | Type | Description |
|---|---|---|
| `file_version_id`| UUID | The file version. |
| `chunk_id` | UUID | The chunk. |
| `workspace_id` | UUID | **Tenant isolation.** |
| `chunk_index` | INT | Order of the chunk in the document. |

### 6. Chat Persistence (The Hybrid Model)

For high-volume data like Chat Sessions, we employ a hybrid persistence strategy.

1.  **Runtime**: Messages are inserted into the `chat_messages` table for O(1) retrieval of structured history during the conversation.
2.  **Persistence**: Simultaneously, the message is formatted as Markdown and appended to `./storage/workspaces/{workspace_id}/latest/chats/<session_id>.md`.
3.  **Consistency**: The `.md` file on disk is the "File Representation" that agents see when they browse directories.

### 7. Remote Files (Object Storage)

For large binary assets (Images, Videos, Archives), storing content directly in PostgreSQL is inefficient.

*   **Redirection**: If `is_remote = true`, the content is stored externally and `hash` may contain an external reference.
*   **Mechanism**: The system uses the version's hash or metadata to fetch the actual payload from Object Storage (S3/Blob).
*   **Use Case**: Large datasets, media files, and high-volume archival data.

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

1.  Query DB for metadata/hash.
2.  Read file from `./storage/workspaces/{workspace_id}/latest/<path>`.

### C. Semantic Search (AI)
"Find files related to 'Quarterly Goals'."

```sql
SELECT f.name, f.slug, f.path, fc.chunk_content, (1 - (fc.embedding <=> '[vector]')) as similarity
FROM file_chunks fc
INNER JOIN file_version_chunks fvc ON fc.id = fvc.chunk_id
INNER JOIN files f ON fvc.file_version_id = f.latest_version_id
WHERE fc.workspace_id = 'current-workspace'
  AND f.deleted_at IS NULL
ORDER BY fc.embedding <=> '[vector]' 
LIMIT 5;
```

### D. Hierarchy Lookup (Materialized Path)
"Get all files in the 'Projects' folder and all its subfolders."

```sql
-- Fast O(log N) lookup without recursion
SELECT * FROM files
WHERE workspace_id = 'current-workspace'
  AND (path = '/projects' OR path LIKE '/projects/%')
  AND deleted_at IS NULL
ORDER BY path ASC;
```
