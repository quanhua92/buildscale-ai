-- Drop revoked_refresh_tokens table (rollback)
DROP INDEX IF EXISTS idx_revoked_tokens_user_id;
DROP INDEX IF EXISTS idx_revoked_tokens_revoked_at;
DROP INDEX IF EXISTS idx_revoked_tokens_token_hash;
DROP TABLE IF EXISTS revoked_refresh_tokens;
