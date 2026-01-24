-- 1. Core File Tables (Identity & Hierarchy)
-- A. files (The Identity Registry)
CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES files(id) ON DELETE CASCADE, -- Hierarchical structure (Folders)
    author_id UUID REFERENCES users(id) ON DELETE SET NULL,
    file_type TEXT NOT NULL,         -- e.g., 'folder', 'document', 'canvas', 'chat'
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'uploading', 'waiting', 'processing', 'ready', 'failed'
    name TEXT NOT NULL,              -- Display name (e.g., 'My Document.md')
    slug TEXT NOT NULL,              -- URL-safe identifier (e.g., 'my-document.md')
    path TEXT NOT NULL,              -- Materialized path for fast tree queries (e.g., '/folder/doc')
    is_virtual BOOLEAN NOT NULL DEFAULT FALSE, -- If true, content is materialized on read
    is_remote BOOLEAN NOT NULL DEFAULT FALSE,  -- If true, content is stored in object storage
    permission INT NOT NULL DEFAULT 600,       -- Unix-style permissions (Owner/Group/World)
    
    -- Cache for the latest version to avoid expensive JOINs/CTEs
    latest_version_id UUID,          -- Populated after the first version is created
    
    deleted_at TIMESTAMPTZ,          -- Soft delete support (Trash bin)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique Constraint: Slugs must be unique within a FOLDER among ACTIVE files.
CREATE UNIQUE INDEX idx_files_slug_active_folder 
ON files(workspace_id, parent_id, slug) 
WHERE parent_id IS NOT NULL AND deleted_at IS NULL;

CREATE UNIQUE INDEX idx_files_slug_active_root 
ON files(workspace_id, slug) 
WHERE parent_id IS NULL AND deleted_at IS NULL;

-- Path uniqueness: Paths must be unique within a workspace among ACTIVE files.
CREATE UNIQUE INDEX idx_files_path_active
ON files(workspace_id, path)
WHERE deleted_at IS NULL;

-- Indexes for performance
CREATE INDEX idx_files_parent ON files(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_files_path_prefix ON files(path text_pattern_ops);
CREATE INDEX idx_files_workspace ON files(workspace_id);
CREATE INDEX idx_files_name ON files(workspace_id, name);
CREATE INDEX idx_files_status ON files(status);
CREATE INDEX idx_files_deleted_at ON files(deleted_at);

-- B. file_versions (The Append-Only Content Store)
CREATE TABLE file_versions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE, -- Tenant isolation
    branch TEXT DEFAULT 'main',      -- Supports parallel drafts or A/B variants
    
    -- JSONB Pillars
    content_raw JSONB NOT NULL,      -- Human source (e.g., Markdown AST, chat log)
    app_data JSONB DEFAULT '{}',     -- Machine metadata (e.g., AI tags, cursor pos, view settings)
    
    hash TEXT NOT NULL,              -- SHA-256 fingerprint for Content-Addressing
    author_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add FK back to versions now that the table exists
ALTER TABLE files ADD CONSTRAINT fk_files_latest_version FOREIGN KEY (latest_version_id) REFERENCES file_versions(id) ON DELETE SET NULL;

CREATE INDEX idx_file_versions_latest ON file_versions(file_id, created_at DESC);
CREATE INDEX idx_file_versions_hash ON file_versions(hash);
CREATE INDEX idx_file_versions_workspace ON file_versions(workspace_id);

-- 2. Relationship & AI Tables
-- C. file_links (The Knowledge Graph)
CREATE TABLE file_links (
    source_file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    target_file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE, -- Tenant isolation
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (source_file_id, target_file_id)
);

CREATE INDEX idx_file_links_target ON file_links(target_file_id);
CREATE INDEX idx_file_links_workspace ON file_links(workspace_id);

-- D. file_chunks (The Semantic Memory)
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE file_chunks (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    chunk_hash TEXT NOT NULL,        -- SHA-256 of chunk_content for deduplication
    chunk_content TEXT NOT NULL,     -- The actual text snippet
    embedding vector(1536),          -- Dimension for OpenAI text-embedding-3-small
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_file_chunks_embedding ON file_chunks 
USING hnsw (embedding vector_cosine_ops);

CREATE UNIQUE INDEX idx_file_chunks_hash ON file_chunks(workspace_id, chunk_hash);
CREATE INDEX idx_file_chunks_workspace ON file_chunks(workspace_id);

-- D2. file_version_chunks (The Linker)
CREATE TABLE file_version_chunks (
    file_version_id UUID NOT NULL REFERENCES file_versions(id) ON DELETE CASCADE,
    chunk_id UUID NOT NULL REFERENCES file_chunks(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE, -- Tenant isolation
    chunk_index INT NOT NULL,        -- To reconstruct order
    PRIMARY KEY (file_version_id, chunk_index)
);

CREATE INDEX idx_file_version_chunks_chunk ON file_version_chunks(chunk_id);
CREATE INDEX idx_file_version_chunks_workspace ON file_version_chunks(workspace_id);

-- E. file_tags (The Taxonomy)
CREATE TABLE file_tags (
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE, -- Tenant isolation
    tag TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (file_id, tag)
);

CREATE INDEX idx_file_tags_tag ON file_tags(tag);
CREATE INDEX idx_file_tags_workspace ON file_tags(workspace_id);
