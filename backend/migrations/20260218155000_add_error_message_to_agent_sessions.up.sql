-- Add error_message column to agent_sessions for persisting error details
ALTER TABLE agent_sessions ADD COLUMN error_message TEXT;

COMMENT ON COLUMN agent_sessions.error_message IS 'Error message when session status is error';
