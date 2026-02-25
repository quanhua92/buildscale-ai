-- Simplified File System Schema (Obsidian-style)
-- Single files table with hash + versions array, archive for backup

CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES files(id) ON DELETE CASCADE,
    file_type TEXT NOT NULL DEFAULT 'document',
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    hash TEXT,  -- SHA-256 of current content (nullable for folders)
    versions TEXT[] DEFAULT '{}',  -- Array of historical hashes (for history traversal)
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_file_type_valid CHECK (file_type IN ('folder', 'document', 'canvas', 'chat', 'whiteboard', 'agent', 'skill', 'plan', 'memory'))
);

-- Path uniqueness within workspace (active files only)
CREATE UNIQUE INDEX idx_files_path_active
ON files(workspace_id, path)
WHERE deleted_at IS NULL;

-- Performance indexes
CREATE INDEX idx_files_parent ON files(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_files_path_prefix ON files(path text_pattern_ops);
CREATE INDEX idx_files_workspace ON files(workspace_id);
CREATE INDEX idx_files_hash ON files(hash);
CREATE INDEX idx_files_deleted_at ON files(deleted_at);

-- GIN index for versions array queries (e.g., "find files that had version X")
CREATE INDEX idx_files_versions ON files USING GIN (versions);
