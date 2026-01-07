use crate::DbConn;
use crate::{
    error::{Error, Result},
    queries::sessions,
};

/// Cleans up all expired sessions from the database
/// This should be called periodically to maintain database performance
pub async fn cleanup_expired_sessions(conn: &mut DbConn) -> Result<u64> {
    let rows_affected = sessions::delete_expired_sessions(conn).await?;
    Ok(rows_affected)
}

/// Revokes all sessions for a specific user
/// This is useful for security operations like password changes or account lockouts
pub async fn revoke_all_user_sessions(conn: &mut DbConn, user_id: uuid::Uuid) -> Result<u64> {
    let rows_affected = sessions::delete_sessions_by_user(conn, user_id).await?;
    Ok(rows_affected)
}

/// Gets all active sessions for a user (non-expired)
pub async fn get_user_active_sessions(conn: &mut DbConn, user_id: uuid::Uuid) -> Result<Vec<crate::models::users::UserSession>> {
    let all_sessions = sessions::get_sessions_by_user(conn, user_id).await?;

    // Filter out expired sessions
    let active_sessions = all_sessions
        .into_iter()
        .filter(|session| session.expires_at > chrono::Utc::now())
        .collect();

    Ok(active_sessions)
}

/// Checks if a user has any active sessions
pub async fn user_has_active_sessions(conn: &mut DbConn, user_id: uuid::Uuid) -> Result<bool> {
    let active_sessions = get_user_active_sessions(conn, user_id).await?;
    Ok(!active_sessions.is_empty())
}

/// Revokes a specific session by its token
pub async fn revoke_session_by_token(conn: &mut DbConn, session_token: &str) -> Result<()> {
    // Validate input
    if session_token.trim().is_empty() {
        return Err(Error::Validation("Session token cannot be empty".to_string()));
    }

    // Hash the token for database lookup
    let token_hash = sessions::hash_session_token(session_token.trim());
    let rows_affected = sessions::delete_session_by_token_hash(conn, &token_hash).await?;

    if rows_affected == 0 {
        return Err(Error::InvalidToken("Session token not found".to_string()));
    }

    Ok(())
}

/// Extends all active sessions for a user by a specified number of hours
pub async fn extend_all_user_sessions(conn: &mut DbConn, user_id: uuid::Uuid, hours_to_extend: i64) -> Result<u64> {
    let active_sessions = get_user_active_sessions(conn, user_id).await?;
    let mut extended_count = 0;

    for session in active_sessions {
        let new_expires_at = chrono::Utc::now() + chrono::Duration::hours(hours_to_extend);
        match sessions::refresh_session(conn, session.id, new_expires_at).await {
            Ok(_) => extended_count += 1,
            Err(_) => {
                // Continue with other sessions even if one fails
                continue;
            }
        }
    }

    Ok(extended_count)
}