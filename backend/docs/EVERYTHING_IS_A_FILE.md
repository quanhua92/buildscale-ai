# Everything is a File: Database Architecture

This document details the "Everything is a file" philosophy and database implementation for BuildScale.ai. This architecture unifies documents, folders, chat sessions, and canvases into a single, cohesive system that supports both hierarchical organization (folders) and network organization (knowledge graph).

## Core Philosophy

In BuildScale.ai, **Identity** is separated from **Content**.

1.  **Identity (`files`)**: The permanent anchor. It has an ID, a name (`slug`), a location (`parent_id`), and an owner. It doesn't change when you edit the document.
2.  **Content (`file_versions`)**: The immutable history. Every "Save" creates a new version. We never update existing content; we only append new versions.

This enables:
*   **Time Travel**: Instantly view the document as it existed at any point in time.
*   **Sync**: Easier conflict resolution (CRDT-friendly).
*   **Audit**: Complete history of who changed what and when.

## Schema Overview

### 1. The Registry: `files`

The `files` table is the central registry for all objects in the system.

| Column | Type | Description |
|---|---|---|
| `id` | UUID (v7) | Unique identifier. |
| `workspace_id` | UUID | Tenant isolation. |
| `parent_id` | UUID | **The Folder Structure.** Points to the parent folder file. `NULL` = Root. |
| `file_type` | TEXT | `document`, `folder`, `canvas`, `chat`, `whiteboard`. |
| `status` | TEXT | `pending`, `uploading`, `waiting`, `processing`, `ready`, `failed`. |
| `slug` | TEXT | URL-safe name. Unique among **active** files in a specific folder. |
| `deleted_at` | TIMESTAMPTZ | **Trash Bin.** If not NULL, the file is in the trash. |
| `created_at` | TIMESTAMPTZ | Creation timestamp. |
| `updated_at` | TIMESTAMPTZ | Last update timestamp (for metadata). |

**Folder Logic:**
*   A **Folder** is just a row in `files` with `file_type = 'folder'`.
*   To put a file in a folder, set its `parent_id` to the folder's `id`.

**Trash Logic:**
*   **Soft Delete**: When a file is deleted, `deleted_at` is set to the current timestamp.
*   **Unique Constraints**: A deleted file releases its claim on the `slug`. You can create a new file with the same name as a deleted one.
*   **Retention**: A background job permanently deletes files where `deleted_at < NOW() - INTERVAL '30 days'`.

### 2. The Content: `file_versions`

Stores the actual data.

| Column | Type | Description |
|---|---|---|
| `file_id` | UUID | Link to the Identity. |
| `content_raw` | JSONB | The payload (Markdown AST, Excalidraw JSON, Chat Array). |
| `app_data` | JSONB | Machine metadata (AI tags, linguistic scores, view settings). |
| `hash` | TEXT | SHA-256 hash of content. Used for deduplication. |
| `branch` | TEXT | Default `main`. Supports A/B variants. |
| `created_at` | TIMESTAMPTZ | Version creation timestamp. |
| `updated_at` | TIMESTAMPTZ | Metadata update timestamp. |

### 3. The Knowledge Graph: `file_links`

Represents connections *between* files, independent of folder structure.

*   **WikiLinks**: `[[Project Alpha]]` in a doc creates a link here.
*   **Dependencies**: "Task A blocks Task B".

| Column | Type | Description |
|---|---|---|
| `source_file_id` | UUID | Where the link originates. |
| `target_file_id` | UUID | What is being linked to. |
| `created_at` | TIMESTAMPTZ | Link creation timestamp. |
| `updated_at` | TIMESTAMPTZ | Metadata update timestamp. |

### 4. The Taxonomy: `file_tags`

High-performance categorization.

| Column | Type | Description |
|---|---|---|
| `file_id` | UUID | The file. |
| `tag` | TEXT | Normalized tag string (e.g., "marketing"). |
| `created_at` | TIMESTAMPTZ | Tag assignment timestamp. |
| `updated_at` | TIMESTAMPTZ | Metadata update timestamp. |

### 5. Semantic Memory: `file_chunks` & `file_version_chunks`

Optimized RAG storage with deduplication.

**`file_chunks` (The Pool):**
Stores unique text snippets and embeddings.
| Column | Type | Description |
|---|---|---|
| `id` | UUID | Chunk ID. |
| `workspace_id` | UUID | Tenant isolation. |
| `chunk_hash` | TEXT | SHA-256 of content. Unique per workspace. |
| `chunk_content`| TEXT | The text snippet. |
| `embedding` | vector | 1536d OpenAI embedding. |

**`file_version_chunks` (The Map):**
Links versions to their chunks.
| Column | Type | Description |
|---|---|---|
| `file_version_id`| UUID | The file version. |
| `chunk_id` | UUID | The chunk. |
| `chunk_index` | INT | Order of the chunk in the document. |

**Optimization Strategy:**
When saving a new version, the system computes hashes for all chunks. It checks `file_chunks` for existing hashes. It only computes embeddings for *new* chunks, reusing IDs for existing ones.

## Common Access Patterns

### A. Folder Navigation (Sidebar)
"Show me everything in the 'Marketing' folder."

```sql
SELECT * FROM files 
WHERE parent_id = 'uuid-of-marketing-folder' 
  AND deleted_at IS NULL -- Important!
ORDER BY slug ASC;
```

### B. Trash Bin
"Show me deleted files."

```sql
SELECT * FROM files 
WHERE workspace_id = 'current-workspace' 
  AND deleted_at IS NOT NULL
ORDER BY deleted_at DESC;
```

### C. Latest Content (Opening a File)
"Get the current content for file X."

```sql
SELECT * FROM file_versions 
WHERE file_id = 'uuid-of-file-x' 
ORDER BY created_at DESC 
LIMIT 1;
```

### D. Semantic Search (AI)
"Find files related to 'Quarterly Goals'."

```sql
SELECT files.slug, file_chunks.chunk_content 
FROM file_chunks
JOIN file_version_chunks ON file_chunks.id = file_version_chunks.chunk_id
JOIN file_versions ON file_version_chunks.file_version_id = file_versions.id
JOIN files ON file_versions.file_id = files.id
WHERE files.workspace_id = 'current-workspace'
  AND files.deleted_at IS NULL -- Exclude trash from AI search
ORDER BY file_chunks.embedding <=> '[vector-from-openai]' 
LIMIT 5;
```

### E. Knowledge Graph
"What files link TO this file? (Backlinks)"

```sql
SELECT source.* 
FROM file_links
JOIN files AS source ON file_links.source_file_id = source.id
WHERE target_file_id = 'current-file-uuid'
  AND source.deleted_at IS NULL;
```

## Ingestion Pipeline & File Lifecycle

The `files` table includes a `status` column to track the lifecycle of a file as it moves through the ingestion pipeline. This ensures the frontend can provide accurate progress feedback.

### Status Flow

1.  **`pending`**: File entry created in DB, but content upload has not started.
2.  **`uploading`**: Client is currently streaming the file binary (or JSON payload).
3.  **`waiting`**: Upload complete. The file is queued for asynchronous processing.
4.  **`processing`**: The background worker is actively parsing, chunking, or embedding the content.
5.  **`ready`**: Processing complete. The file is fully indexed, searchable, and safe to read.
6.  **`failed`**: An error occurred during upload or processing. Check `app_data` for error details.

### Pipeline Stages

The background worker performs the following tasks during the `processing` state:

*   **Link Extraction**: Parses `[[WikiLinks]]` from text/markdown and updates `file_links`.
*   **Vector Chunking**: Splits text into chunks, computes OpenAI embeddings, and updates `file_chunks`.
*   **Media Analysis**: (Future) OCR for PDFs, transcription for audio.

### Distributed Ingestion Strategy (Future)
To handle high-load ingestion, we will implement a Transactional Outbox pattern:
*   **Tasks Table**: A future `tasks` table will queue jobs atomically when a file enters `waiting` status.
*   **GPU Workers**: External workers will pull tasks, perform heavy AI processing (embedding/OCR), and update the file to `ready`.
