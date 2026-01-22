# Implementation Plan: "Everything is a File" System

**Status**: 🚧 In Progress
**Architecture Reference**: `backend/docs/EVERYTHING_IS_A_FILE.md`
**Database Schema**: `backend/migrations/20260115134428_files_schema.up.sql`

This document serves as the self-contained execution guide for implementing the unified file system in BuildScale.ai. It tracks the journey from database models to a full AI-ready API.

---

## 📅 Progress Log

- [x] **Phase 1**: Database & Domain Modeling (The Skeleton)
- [x] **Phase 2**: Core Versioning Logic (The Heart)
- [x] **Phase 3**: Basic API & File Lifecycle (The Interface)
- [x] **Phase 4**: Advanced Organization (The Folder Tree & Trash)
- [x] **Phase 5**: Knowledge Graph & Taxonomy (The Network)
- [ ] **Phase 6**: AI Foundation (The Brain)

---

## Phase 1: Database & Domain Modeling (The Skeleton)
**Goal**: Establish the Rust structures that mirror the PostgreSQL schema. Ensure `sqlx` can talk to the new tables.

- [x] **1.1 Dependencies**
    - [x] Add `pgvector` to `Cargo.toml` (needed for `file_chunks` embedding).
    - [x] Add `strum` / `strum_macros` for Enum string serialization (optional but recommended for `FileType`).
- [x] **1.2 Domain Models (`src/models/files.rs`)**
    - [x] Define Enums: `FileStatus` (Pending, Uploading...), `FileType` (Document, Folder...).
    - [x] Struct `File` (Identity): `id`, `parent_id`, `slug`, `status`, `created_at`...
    - [x] Struct `FileVersion` (Content): `id`, `file_id`, `content_raw` (JSONB), `hash`, `branch`.
    - [x] Struct `FileChunk` (Semantic): `id`, `chunk_hash`, `embedding` (Vector).
    - [x] Struct `FileLink` & `FileTag`.
- [x] **1.3 Base Queries (`src/queries/files.rs`)**
    - [x] `create_file_identity`: Insert into `files` table.
    - [x] `get_file_by_id`: Fetch basic metadata.
    - [x] `get_file_by_slug`: Resolve path (`parent_id` + `slug`).

## Phase 2: Core Versioning Logic (The Heart)
**Goal**: Implement the "Identity vs Content" philosophy. Files are never overwritten, only appended.

- [x] **2.1 Service Layer (`src/services/files.rs`)**
    - [x] **Hashing Logic**: Implement SHA-256 calculation for content.
    - [x] **Storage Strategy**:
        - *Logic*: If content is small text (<1MB), store in `content_raw`.
        - *Future Stub*: If binary/large, upload to S3 (mock for now) and store pointer.
    - [x] `create_version`:
        - Input: `file_id`, `content`, `author`.
        - Action: Calculate Hash -> Check Dedup (optional) -> Insert `file_versions` -> Update `files.updated_at`.
- [x] **2.2 Transactional Creation**
    - [x] `create_file_with_content`:
        - Run inside `sqlx::Transaction`.
        - 1. Create `File` (Identity).
        - 2. Create first `FileVersion` (Content).
        - 3. Commit.

## Phase 3: Basic API & File Lifecycle (The Interface)
**Goal**: Expose creation and reading via REST API.

- [x] **3.1 API Handlers (`src/handlers/files.rs`)**
    - [x] `POST /workspaces/:id/files`:
        - Body: `{ parent_id, name, file_type, initial_content }`.
        - Logic: Sanitize slug -> Call Service -> Return 201.
    - [x] `GET /workspaces/:id/files/:file_id`:
        - Return `File` metadata + `LatestVersion` content.
- [x] **3.2 Ingestion State Machine**
    - [x] Handle `status` transitions:
        - `pending` (Created) -> `ready` (Saved).
        - (Future hooks for `processing` will go here).

## Phase 4: Advanced Organization (The Folder Tree & Trash)
**Goal**: Manage the hierarchy and safety mechanisms.

- [x] **4.1 Folder Navigation**
    - [x] Query: `list_files(parent_id)`:
        - Filter: `deleted_at IS NULL`.
        - Order: Folders first, then Alphabetical.
    - [x] Handler: `GET /workspaces/:id/files?parent_id=...`.
- [x] **4.2 Move & Rename**
    - [x] Query: `update_file_location(id, new_parent, new_slug)`.
    - [x] **Validation Rule**: Cycle Detection (Ensure Target Folder is not a child of Source).
    - [x] **Validation Rule**: Slug Uniqueness (Check collision in Target Folder).
    - [x] Handler: `PATCH /workspaces/:id/files/:file_id`.
- [x] **4.3 Trash Bin (Soft Delete)**
    - [x] Query: `soft_delete(id)`: Set `deleted_at = NOW()`.
    - [x] Query: `restore(id)`: Set `deleted_at = NULL`.
    - [x] Handler: `DELETE ...` and `POST .../restore`.
    - [x] **Constraint**: Folder must be empty before deletion.

## Phase 5: Knowledge Graph & Taxonomy (The Network)
**Goal**: Connect files beyond folders.

- [x] **5.1 Tagging**
    - [x] Handlers: `POST /files/:id/tags`, `DELETE /files/:id/tags`.
    - [x] Query: `get_files_by_tag(tag)`.
- [x] **5.2 Links (Backlinks)**
    - [x] Query: `get_backlinks(file_id)`: Find all files that link TO this file.
    - [x] Handler: `GET /files/:id/backlinks`.
    - [x] Service: Workspace boundary check for linking.


## Phase 6: AI Foundation (The Brain)
**Goal**: Prepare data for the RAG engine.

- [ ] **6.1 Chunking Stub**
    - [ ] Implement `FileChunk` insertion logic in `src/queries/files.rs`.
    - [ ] Create a placeholder function `process_file_for_ai(file_id)` in `services/files.rs`.
    - [ ] *Note*: Actual OpenAI integration can be a separate task, but the DB plumbing happens here.

## 🏁 Definition of Done
1.  All Rust structs match the Migration schema.
2.  Can create a Folder, File, and add Versions via API.
3.  Can Move/Rename files safely.
4.  Can "Soft Delete" and see files in Trash.
5.  `cargo test` passes for new logic.
