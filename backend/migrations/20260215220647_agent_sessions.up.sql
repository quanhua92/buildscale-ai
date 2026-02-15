-- Agent sessions table for tracking active AI agent instances
CREATE TABLE agent_sessions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    chat_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    agent_type TEXT NOT NULL, -- 'assistant', 'planner', 'builder'
    status TEXT NOT NULL DEFAULT 'idle', -- 'idle', 'running', 'paused', 'completed', 'error'
    model TEXT NOT NULL,
    mode TEXT NOT NULL DEFAULT 'chat', -- 'chat', 'plan', 'build'
    current_task TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_heartbeat TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    UNIQUE(chat_id)
);

-- Indexes for querying active sessions
CREATE INDEX idx_agent_sessions_workspace_id ON agent_sessions(workspace_id);
CREATE INDEX idx_agent_sessions_user_id ON agent_sessions(user_id);
CREATE INDEX idx_agent_sessions_status ON agent_sessions(status);
CREATE INDEX idx_agent_sessions_last_heartbeat ON agent_sessions(last_heartbeat);
CREATE INDEX idx_agent_sessions_agent_type ON agent_sessions(agent_type);

-- Add comments for documentation
COMMENT ON TABLE agent_sessions IS 'Tracks active AI agent sessions across workspaces for visibility and distributed agent swarm support';
COMMENT ON COLUMN agent_sessions.id IS 'Unique session identifier using UUID v7 for time-ordered IDs';
COMMENT ON COLUMN agent_sessions.workspace_id IS 'Workspace this session belongs to';
COMMENT ON COLUMN agent_sessions.chat_id IS 'Chat file this session is associated with (unique per session)';
COMMENT ON COLUMN agent_sessions.user_id IS 'User who initiated this session';
COMMENT ON COLUMN agent_sessions.agent_type IS 'Type of agent: assistant, planner, or builder';
COMMENT ON COLUMN agent_sessions.status IS 'Current session status: idle, running, paused, completed, error';
COMMENT ON COLUMN agent_sessions.model IS 'AI model being used (e.g., claude-3-5-sonnet, gpt-4o)';
COMMENT ON COLUMN agent_sessions.mode IS 'Operating mode: chat, plan, or build';
COMMENT ON COLUMN agent_sessions.current_task IS 'Description of the current task being executed';
COMMENT ON COLUMN agent_sessions.last_heartbeat IS 'Last heartbeat timestamp for detecting stale sessions';
COMMENT ON COLUMN agent_sessions.completed_at IS 'When the session completed (null if still active)';
