use crate::{
    error::{Error, Result},
    models::users::{NewUserSession, UpdateUserSession, UserSession},
};
use chrono::Utc;
use uuid::Uuid;

use crate::DbConn;

/// Creates a new user session in the database.
pub async fn create_session(conn: &mut DbConn, new_session: NewUserSession) -> Result<UserSession> {
    let session = sqlx::query_as!(
        UserSession,
        r#"
        INSERT INTO user_sessions (user_id, token, expires_at)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, token, expires_at, created_at, updated_at
        "#,
        new_session.user_id,
        new_session.token,
        new_session.expires_at
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(session)
}

/// Gets a single session by its token. The session may not exist.
pub async fn get_session_by_token(conn: &mut DbConn, token: &str) -> Result<Option<UserSession>> {
    let session = sqlx::query_as!(
        UserSession,
        r#"
        SELECT id, user_id, token, expires_at, created_at, updated_at
        FROM user_sessions
        WHERE token = $1
        "#,
        token
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
        SELECT id, user_id, token, expires_at, created_at, updated_at
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
        RETURNING id, user_id, token, expires_at, created_at, updated_at
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

/// Deletes a session by its token.
pub async fn delete_session_by_token(conn: &mut DbConn, token: &str) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM user_sessions
        WHERE token = $1
        "#,
    )
    .bind(token)
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
pub async fn is_session_valid(conn: &mut DbConn, token: &str) -> Result<bool> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM user_sessions
        WHERE token = $1 AND expires_at > NOW()
        "#,
        token
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(count > 0)
}

/// Gets a valid session (exists and not expired) by its token.
pub async fn get_valid_session_by_token(conn: &mut DbConn, token: &str) -> Result<Option<UserSession>> {
    let session = sqlx::query_as!(
        UserSession,
        r#"
        SELECT id, user_id, token, expires_at, created_at, updated_at
        FROM user_sessions
        WHERE token = $1 AND expires_at > NOW()
        "#,
        token
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
        RETURNING id, user_id, token, expires_at, created_at, updated_at
        "#,
        new_expires_at,
        session_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(updated_session)
}