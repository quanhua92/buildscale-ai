use crate::{
    error::{Error, Result},
    models::users::{NewUserSession, UpdateUserSession, UserSession, RevokedRefreshToken},
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::DbConn;

/// Hash a session token using SHA-256 for secure storage
pub fn hash_session_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Creates a new user session in the database.
pub async fn create_session(conn: &mut DbConn, new_session: NewUserSession) -> Result<UserSession> {
    let session = sqlx::query_as!(
        UserSession,
        r#"
        INSERT INTO user_sessions (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, token_hash, expires_at, created_at, updated_at
        "#,
        new_session.user_id,
        new_session.token_hash,
        new_session.expires_at
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(session)
}

/// Gets a single session by its token hash. The session may not exist.
pub async fn get_session_by_token_hash(conn: &mut DbConn, token_hash: &str) -> Result<Option<UserSession>> {
    let session = sqlx::query_as!(
        UserSession,
        r#"
        SELECT id, user_id, token_hash, expires_at, created_at, updated_at
        FROM user_sessions
        WHERE token_hash = $1
        "#,
        token_hash
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(session)
}

/// Gets all sessions for a specific user.
pub async fn get_sessions_by_user(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<UserSession>> {
    let sessions = sqlx::query_as!(
        UserSession,
        r#"
        SELECT id, user_id, token_hash, expires_at, created_at, updated_at
        FROM user_sessions
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(sessions)
}

/// Updates an existing session's details.
pub async fn update_session(conn: &mut DbConn, session_id: Uuid, update: UpdateUserSession) -> Result<UserSession> {
    let updated_session = sqlx::query_as!(
        UserSession,
        r#"
        UPDATE user_sessions
        SET expires_at = COALESCE($1, expires_at), updated_at = now()
        WHERE id = $2
        RETURNING id, user_id, token_hash, expires_at, created_at, updated_at
        "#,
        update.expires_at,
        session_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(updated_session)
}

/// Deletes a session by its ID.
pub async fn delete_session(conn: &mut DbConn, session_id: Uuid) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM user_sessions
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}

/// Deletes a session by its token hash.
pub async fn delete_session_by_token_hash(conn: &mut DbConn, token_hash: &str) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM user_sessions
        WHERE token_hash = $1
        "#,
    )
    .bind(token_hash)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}

/// Deletes all sessions for a specific user.
pub async fn delete_sessions_by_user(conn: &mut DbConn, user_id: Uuid) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM user_sessions
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}

/// Deletes all expired sessions.
pub async fn delete_expired_sessions(conn: &mut DbConn) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM user_sessions
        WHERE expires_at < NOW()
        "#,
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}

/// Checks if a session is valid (exists and not expired).
pub async fn is_session_valid(conn: &mut DbConn, token_hash: &str) -> Result<bool> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM user_sessions
        WHERE token_hash = $1 AND expires_at > NOW()
        "#,
        token_hash
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(count > 0)
}

/// Gets a valid session (exists and not expired) by its token hash.
pub async fn get_valid_session_by_token_hash(conn: &mut DbConn, token_hash: &str) -> Result<Option<UserSession>> {
    let session = sqlx::query_as!(
        UserSession,
        r#"
        SELECT id, user_id, token_hash, expires_at, created_at, updated_at
        FROM user_sessions
        WHERE token_hash = $1 AND expires_at > NOW()
        "#,
        token_hash
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(session)
}

/// Refreshes a session by extending its expiration time.
pub async fn refresh_session(conn: &mut DbConn, session_id: Uuid, new_expires_at: chrono::DateTime<Utc>) -> Result<UserSession> {
    let updated_session = sqlx::query_as!(
        UserSession,
        r#"
        UPDATE user_sessions
        SET expires_at = $1, updated_at = now()
        WHERE id = $2
        RETURNING id, user_id, token_hash, expires_at, created_at, updated_at
        "#,
        new_expires_at,
        session_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(updated_session)
}

/// Updates a session's token hash in the database.
///
/// This function supports refresh token rotation by replacing the old
/// token hash with a new one, maintaining the session ID and expiration.
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - ID of the session to update
/// * `new_token_hash` - New SHA-256 hashed refresh token
///
/// # Returns
/// Result containing the updated session or error
///
/// # Errors
/// Returns error if session not found or database operation fails
pub async fn update_session_token_hash(
    conn: &mut DbConn,
    session_id: Uuid,
    new_token_hash: &str,
) -> Result<UserSession> {
    let updated_session = sqlx::query_as!(
        UserSession,
        r#"
        UPDATE user_sessions
        SET token_hash = $1, updated_at = now()
        WHERE id = $2
        RETURNING id, user_id, token_hash, expires_at, created_at, updated_at
        "#,
        new_token_hash,
        session_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(updated_session)
}
// ============================================================================
// REVOKED TOKEN MANAGEMENT (Stolen Token Detection)
// ============================================================================

/// Records a revoked refresh token for theft detection
pub async fn create_revoked_token(
    conn: &mut DbConn,
    user_id: Uuid,
    token_hash: &str,
) -> Result<RevokedRefreshToken> {
    let revoked_token = sqlx::query_as!(
        RevokedRefreshToken,
        r#"
        INSERT INTO revoked_refresh_tokens (user_id, token_hash)
        VALUES ($1, $2)
        RETURNING id, user_id, token_hash, revoked_at, reason
        "#,
        user_id,
        token_hash
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(revoked_token)
}

/// Checks if a token has been revoked (indicating potential theft)
pub async fn get_revoked_token(
    conn: &mut DbConn,
    token_hash: &str,
) -> Result<Option<RevokedRefreshToken>> {
    let revoked_token = sqlx::query_as!(
        RevokedRefreshToken,
        r#"
        SELECT id, user_id, token_hash, revoked_at, reason
        FROM revoked_refresh_tokens
        WHERE token_hash = $1
        "#,
        token_hash
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(revoked_token)
}

/// Deletes all revoked tokens for a specific user
pub async fn delete_revoked_tokens_by_user(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM revoked_refresh_tokens
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}

/// Deletes expired revoked tokens (older than grace period)
/// Should be called periodically to maintain table size
pub async fn delete_expired_revoked_tokens(
    conn: &mut DbConn,
    grace_period_minutes: i64,
) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM revoked_refresh_tokens
        WHERE revoked_at < NOW() - (INTERVAL '1 minute' * $1)
        "#,
    )
    .bind(grace_period_minutes)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}
