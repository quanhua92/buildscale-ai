-- Tags table for tag indexing
-- This table is hydrated by a background worker when files change

CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure unique tag per file per workspace
    CONSTRAINT tags_workspace_file_tag_unique UNIQUE (workspace_id, file_id, tag)
);

-- Index for fast tag lookups
CREATE INDEX idx_tags_workspace_tag ON tags(workspace_id, tag);
CREATE INDEX idx_tags_file ON tags(file_id);

COMMENT ON TABLE tags IS 'Index of tags extracted from file content for fast lookups';
COMMENT ON COLUMN tags.tag IS 'Lowercase tag name without # prefix (e.g., "work", "project/alpha")';
