-- User authentication sessions for maintaining login state
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for fast session token lookup
CREATE INDEX idx_user_sessions_token_hash ON user_sessions(token_hash);

-- Index for finding user's sessions and cleanup
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);

-- Index for cleanup of expired sessions
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);