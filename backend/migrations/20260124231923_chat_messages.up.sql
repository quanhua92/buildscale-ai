-- 20260124231923_chat_messages.up.sql
CREATE TABLE chat_messages (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    role TEXT NOT NULL,         -- Managed by Rust Enum: user, assistant, system, tool
    content TEXT NOT NULL,      -- Message body (Markdown)
    metadata JSONB NOT NULL DEFAULT '{}', -- Attachments (files, agents, urls, skills)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

-- Index for chronological retrieval of chat history for a specific session
CREATE INDEX idx_chat_messages_file_history ON chat_messages(file_id, created_at ASC) WHERE deleted_at IS NULL;

-- Index for tenant-isolated message searching/listing
CREATE INDEX idx_chat_messages_workspace ON chat_messages(workspace_id);

-- GIN index for metadata searching (e.g., finding messages with specific attachments)
CREATE INDEX idx_chat_messages_metadata ON chat_messages USING GIN (metadata);
