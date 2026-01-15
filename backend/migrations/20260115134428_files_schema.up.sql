-- 1. Core File Tables (Identity & Hierarchy)
-- A. files (The Identity Registry)
CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES files(id) ON DELETE CASCADE, -- Hierarchical structure (Folders)
    author_id UUID NOT NULL REFERENCES users(id),
    file_type TEXT NOT NULL,         -- e.g., 'folder', 'document', 'canvas', 'chat'
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'uploading', 'waiting', 'processing', 'ready', 'failed'
    slug TEXT NOT NULL,              -- The unique identifier/filename within the folder
    deleted_at TIMESTAMPTZ,          -- Soft delete support (Trash bin)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique Constraint: Slugs must be unique within a FOLDER among ACTIVE files.
-- 1. For files inside a folder (parent_id is NOT NULL)
CREATE UNIQUE INDEX idx_files_slug_active_folder 
ON files(workspace_id, parent_id, slug) 
WHERE parent_id IS NOT NULL AND deleted_at IS NULL;

-- 2. For files in the root directory (parent_id is NULL)
CREATE UNIQUE INDEX idx_files_slug_active_root 
ON files(workspace_id, slug) 
WHERE parent_id IS NULL AND deleted_at IS NULL;

-- Index for Sidebar/Folder navigation (finding all ACTIVE files in a specific folder)
CREATE INDEX idx_files_parent ON files(parent_id) WHERE deleted_at IS NULL;

-- Index for workspace-wide listing
CREATE INDEX idx_files_workspace ON files(workspace_id);
-- Index for filtering by status (e.g. finding stuck uploads)
CREATE INDEX idx_files_status ON files(status);
-- Index for Trash management (finding files eligible for permanent deletion)
CREATE INDEX idx_files_deleted_at ON files(deleted_at);

-- B. file_versions (The Append-Only Content Store)
CREATE TABLE file_versions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    branch TEXT DEFAULT 'main',      -- Supports parallel drafts or A/B variants
    
    -- JSONB Pillars
    content_raw JSONB NOT NULL,      -- Human source (e.g., Markdown AST, chat log)
    app_data JSONB DEFAULT '{}',     -- Machine metadata (e.g., AI tags, cursor pos, view settings)
    
    hash TEXT NOT NULL,              -- SHA-256 fingerprint for Content-Addressing
    author_id UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index to instantly fetch the most recent version of a file
CREATE INDEX idx_file_versions_latest ON file_versions(file_id, created_at DESC);
-- Index for finding specific content hashes (Deduplication)
CREATE INDEX idx_file_versions_hash ON file_versions(hash);

-- 2. Relationship & AI Tables
-- C. file_links (The Knowledge Graph)
CREATE TABLE file_links (
    source_file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    target_file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (source_file_id, target_file_id)
);

-- Index for "Backlinks" (finding what files link TO this file)
CREATE INDEX idx_file_links_target ON file_links(target_file_id);

-- D. file_chunks (The Semantic Memory)
-- Extension required for vector search (enabled in previous migration)
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE file_chunks (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    chunk_hash TEXT NOT NULL,        -- SHA-256 of chunk_content for deduplication
    chunk_content TEXT NOT NULL,     -- The actual text snippet
    embedding vector(1536),          -- Dimension for OpenAI text-embedding-3-small
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- HNSW index for high-performance semantic search (Cosine Similarity)
CREATE INDEX idx_file_chunks_embedding ON file_chunks 
USING hnsw (embedding vector_cosine_ops);

-- Index for content-addressable deduplication (reuse chunks across files)
CREATE UNIQUE INDEX idx_file_chunks_hash ON file_chunks(workspace_id, chunk_hash);

-- D2. file_version_chunks (The Linker)
-- Junction table mapping versions to chunks (Many-to-Many for efficient reuse)
CREATE TABLE file_version_chunks (
    file_version_id UUID NOT NULL REFERENCES file_versions(id) ON DELETE CASCADE,
    chunk_id UUID NOT NULL REFERENCES file_chunks(id) ON DELETE CASCADE,
    chunk_index INT NOT NULL,        -- To reconstruct order
    PRIMARY KEY (file_version_id, chunk_index)
);

-- Index to find which versions use a specific chunk
CREATE INDEX idx_file_version_chunks_chunk ON file_version_chunks(chunk_id);

-- E. file_tags (The Taxonomy)
CREATE TABLE file_tags (
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (file_id, tag)
);

-- Index for fast "Find by tag" queries
CREATE INDEX idx_file_tags_tag ON file_tags(tag);
