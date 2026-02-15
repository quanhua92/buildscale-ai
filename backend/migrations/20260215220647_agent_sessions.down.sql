-- Drop indexes first (they will be dropped automatically with table, but being explicit)
DROP INDEX IF EXISTS idx_agent_sessions_agent_type;
DROP INDEX IF EXISTS idx_agent_sessions_last_heartbeat;
DROP INDEX IF EXISTS idx_agent_sessions_status;
DROP INDEX IF EXISTS idx_agent_sessions_user_id;
DROP INDEX IF EXISTS idx_agent_sessions_workspace_id;

-- Drop the agent_sessions table
DROP TABLE IF EXISTS agent_sessions;
