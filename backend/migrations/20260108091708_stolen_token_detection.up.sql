-- Track revoked refresh tokens for theft detection
-- This table stores temporarily revoked refresh tokens to detect token theft
-- Tokens are cleaned up after grace period (5 minutes)

CREATE TABLE revoked_refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    revoked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reason TEXT NOT NULL DEFAULT 'token_rotation'
);

-- Index for fast stolen token lookup (used in every refresh)
CREATE INDEX idx_revoked_tokens_token_hash ON revoked_refresh_tokens(token_hash);

-- Index for time-based cleanup (remove tokens older than grace period)
CREATE INDEX idx_revoked_tokens_revoked_at ON revoked_refresh_tokens(revoked_at);

-- Index for user-level revocation queries (security operations)
CREATE INDEX idx_revoked_tokens_user_id ON revoked_refresh_tokens(user_id);

COMMENT ON TABLE revoked_refresh_tokens IS 'Stores temporarily revoked refresh tokens to detect token theft. Tokens are cleaned up after grace period (5 minutes).';
