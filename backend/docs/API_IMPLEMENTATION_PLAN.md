# TODO: Implement "Everything is a File" REST API

This document outlines the implementation plan for the REST API supporting the "Everything is a file" architecture.

## 1. API Structure & Models

- [ ] **Define Data Models (`src/models/files.rs`)**
    - [ ] `File` struct (Identity)
        - Fields: `id`, `slug`, `file_type`, `parent_id`, `status` (pending, uploading, waiting, processing, ready, failed)
    - [ ] `FileVersion` struct (Content)
    - [ ] `FileChunk` & `FileLink` structs
    - [ ] Request/Response DTOs:
        - `CreateFileRequest` (parent_id, slug, file_type)
        - `UpdateFileRequest` (parent_id, slug - for move/rename)
        - `CreateVersionRequest` (content_raw, app_data)

- [ ] **Define Service Layer (`src/services/files.rs`)**
    - [ ] `create_file` (Handle uniqueness, slug generation)
    - [ ] `move_file` (Handle parent_id updates)
    - [ ] `soft_delete_file` (Set `deleted_at`)
    - [ ] `restore_file` (Unset `deleted_at`)
    - [ ] `create_version` (Insert immutable version, hash content)
    - [ ] `get_latest_version` (Fetch most recent content)

## 2. Core Endpoints (Registry & Hierarchy)

- [ ] **POST `/workspaces/:workspace_id/files`**
    - Create a file/folder anchor.
    - Validate `slug` uniqueness within `parent_id`.

- [ ] **PATCH `/workspaces/:workspace_id/files/:file_id`**
    - Rename (update `slug`) or Move (update `parent_id`).
    - Enforce circular dependency checks (folder cannot move into itself).

- [ ] **DELETE `/workspaces/:workspace_id/files/:file_id`**
    - Soft delete (set `deleted_at`).
    - Optional: `?permanent=true` for admin cleanup.

- [ ] **GET `/workspaces/:workspace_id/files`**
    - List contents of a folder (`?parent_id=...`).
    - Filter `deleted_at IS NULL` by default.

- [ ] **GET `/workspaces/:workspace_id/files/:file_id`**
    - Retrieve metadata + latest version content.
    - **Smart Retrieval**:
        - If content is Text/JSON: Return directly in response.
        - If content is S3 Pointer: Generate and return a **Pre-signed URL** for secure client-side download.

## 3. Versioning & Binary Uploads

- [ ] **POST `/workspaces/:workspace_id/files/:file_id/versions`**
    - **Purpose**: For *Application State* saves (Editor save, Canvas save).
    - **Payload**: JSON body `{ "content_raw": {...}, "app_data": {...} }`.
    - **Logic**: Direct insert into `file_versions`.
    - **Trigger**: Background job for Link Parsing & Chunking.
    - **Status Tracking**: Update `file.status` to `waiting` (queued) -> `processing` -> `ready`.

- [ ] **POST `/workspaces/:workspace_id/files/:file_id/upload`**
    - **Purpose**: For *File Imports* (User uploads a file from disk).
    - **Payload**: `multipart/form-data`.
    - **Smart Logic**:
        1.  **Stream & Hash**: Compute SHA-256 while streaming.
        2.  **Decision Gate**:
            - **Case A: Small Text/Code** (e.g., .md, .txt, .csv < 1MB):
                - Read content into memory.
                - Store *directly* in `content_raw` (e.g., `{ "text": "..." }`).
                - Enables DB-level search/diffing without S3 fetch.
            - **Case B: Binary / Large File** (e.g., .png, .pdf, large .log):
                - Upload to S3 (Key = SHA-256 Hash).
                - Store *Pointer* in `content_raw` (e.g., `{ "s3_key": "...", "mime": "...", "size": ... }`).
    - **Result**: Unified endpoint for all uploads; system optimizes storage strategy.

## 4. AI & Search Integration

- [ ] **Ingestion Pipeline (Background Worker)**
    - [ ] **Link Extractor**: Parse `[[WikiLinks]]` -> Insert `file_links`.
    - [ ] **Chunker**: Split content -> Compute Embeddings -> Insert `file_chunks` & `file_version_chunks`.

- [ ] **GET `/workspaces/:workspace_id/search/semantic`**
    - Input: `?q=query_string`
    - Logic: Embed query -> Vector Search (`pgvector`) -> Return relevant file snippets.

## 5. Security & Access Control

- [ ] **Middleware / Guards**
    - Ensure `workspace_id` in URL matches authenticated user's permissions.
    - Role-based checks (Viewers can read, Editors can version).

## 6. Testing

- [ ] **Unit Tests**: Slug collision logic, Version hashing.
- [ ] **Integration Tests**: Folder tree navigation, Full upload flow.
