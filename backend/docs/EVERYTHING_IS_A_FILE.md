← [Back to Index](./README.md) | **The Vision**: [Files Are All You Need](./FILES_ARE_ALL_YOU_NEED.md)

# Everything is a File: Database Architecture

This document details the "Everything is a file" philosophy and database implementation for BuildScale.ai. This architecture unifies documents, folders, chat sessions, and canvases into a single, cohesive system that supports both hierarchical organization (folders) and network organization (knowledge graph).

## Core Philosophy

In BuildScale.ai, **Identity** is separated from **Content**.

1.  **Identity (`files`)**: The permanent anchor. It has an ID, a human name (`name`), a machine identifier (`slug`), a location (`parent_id`), and an owner. It doesn't change when you edit the document content.
2.  **Content (`file_versions`)**: The immutable history. Every "Save" creates a new version. We never update existing content; we only append new versions.

**Default Type & Validation**:
*   The system defaults to `file_type = 'document'` if not specified.
*   **Documents** are strictly validated to ensure they contain a `text` field with string content.
*   Specialized types like `canvas` or `chat` allow arbitrary JSON structures but must be valid JSON.
*   **Folders** are identity-only nodes and do not typically hold text content.

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
| `is_virtual` | BOOLEAN | **Dynamic Content.** If true, content is generated on-the-fly (e.g. from `chat_messages` table) rather than stored in `file_versions`. |
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
*   **Retention**: A background job permanently deletes files where `deleted_at < NOW() - INTERVAL '30 days'`.

### 2. The Content: `file_versions`

Stores the actual data.

**Architectural Note**:
*   Small content (Markdown, Code, JSON) is stored directly in `content_raw` (JSONB) in PostgreSQL for atomic versioning and instant access.
*   Large blobs (Images, Videos, Archives) are typically stored in S3/Object Storage, with only the reference URL and metadata stored in PostgreSQL. This hybrid approach enables both high-performance metadata operations and infinite storage scaling.

| Column | Type | Description |
|---|---|---|
| `id` | UUID | Unique version identifier. |
| `file_id` | UUID | Link to the Identity. |
| `workspace_id` | UUID | **Tenant isolation.** Denormalized for performance. |
| `author_id` | UUID | Who created this specific version. Supports user deletion. |
| `content_raw` | JSONB | The payload (Markdown AST, Excalidraw JSON, Chat Array). |
| `app_data` | JSONB | Machine metadata (AI tags, linguistic scores, view settings). |
| `hash` | TEXT | SHA-256 hash of content. Used for deduplication. |
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

### 6. Virtual Files (Dynamic Content)

For high-volume data like Chat Sessions, storing every single message as a new `file_version` blob would be inefficient.

*   **`is_virtual = true`**: Indicates the file's content is not stored in `file_versions`.
*   **Mechanism**: When read, the system dynamically materializes the content from a specialized source (e.g., a `chat_messages` table).
*   **Use Case**: Infinite chat histories, real-time logs, or computed views.

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

```sql
-- O(1) Lookup using the cache
SELECT fv.* 
FROM file_versions fv
JOIN files f ON f.latest_version_id = fv.id
WHERE f.id = 'uuid-of-file-x';
```

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

