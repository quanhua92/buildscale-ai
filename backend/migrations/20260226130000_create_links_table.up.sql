-- Links table for Obsidian-style link indexing
-- This table is hydrated by a background worker when files change

CREATE TABLE links (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    source_file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    target_name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure unique link per file
    CONSTRAINT links_workspace_source_target_unique UNIQUE (workspace_id, source_file_id, target_name)
);

-- Index for fast backlink lookups (find files linking TO a target)
CREATE INDEX idx_links_target ON links(workspace_id, target_name);
-- Index for finding all links FROM a file
CREATE INDEX idx_links_source ON links(source_file_id);

COMMENT ON TABLE links IS 'Index of wikilinks extracted from file content for fast lookups';
COMMENT ON COLUMN links.target_name IS 'Lowercase link target name without [[ ]] (e.g., "note", "project/roadmap")';
