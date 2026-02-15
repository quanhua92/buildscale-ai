use crate::{
    error::{Error, Result},
    models::agent_session::{AgentSession, AgentType, NewAgentSession, SessionStatus},
};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::DbConn;

/// Default heartbeat timeout in seconds - sessions older than this are considered stale
pub const STALE_SESSION_THRESHOLD_SECONDS: i64 = 120; // 2 minutes

/// Creates a new agent session in the database.
pub async fn create_session(conn: &mut DbConn, new_session: NewAgentSession) -> Result<AgentSession> {
    let session = sqlx::query_as!(
        AgentSession,
        r#"
        INSERT INTO agent_sessions (
            workspace_id, chat_id, user_id, agent_type, status, model, mode
        )
        VALUES ($1, $2, $3, $4, 'idle', $5, $6)
        RETURNING
            id,
            workspace_id,
            chat_id,
            user_id,
            agent_type as "agent_type: AgentType",
            status as "status: SessionStatus",
            model,
            mode,
            current_task,
            created_at,
            updated_at,
            last_heartbeat,
            completed_at
        "#,
        new_session.workspace_id,
        new_session.chat_id,
        new_session.user_id,
        new_session.agent_type as AgentType,
        new_session.model,
        new_session.mode,
    )
    .fetch_one(conn)
    .await
    .map_err(|e| {
        let error_msg = e.to_string().to_lowercase();

        // Check for unique constraint violations on chat_id
        if error_msg.contains("unique")
            || error_msg.contains("duplicate key")
            || error_msg.contains("agent_sessions_chat_id_key")
        {
            Error::Conflict(format!(
                "An active session already exists for this chat: {}",
                new_session.chat_id
            ))
        } else {
            Error::Sqlx(e)
        }
    })?;

    Ok(session)
}

/// Gets a single session by its ID.
pub async fn get_session_by_id(conn: &mut DbConn, id: Uuid) -> Result<Option<AgentSession>> {
    let session = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            id,
            workspace_id,
            chat_id,
            user_id,
            agent_type as "agent_type: AgentType",
            status as "status: SessionStatus",
            model,
            mode,
            current_task,
            created_at,
            updated_at,
            last_heartbeat,
            completed_at
        FROM agent_sessions
        WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(session)
}

/// Gets a session by its chat ID (unique).
pub async fn get_session_by_chat(conn: &mut DbConn, chat_id: Uuid) -> Result<Option<AgentSession>> {
    let session = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            id,
            workspace_id,
            chat_id,
            user_id,
            agent_type as "agent_type: AgentType",
            status as "status: SessionStatus",
            model,
            mode,
            current_task,
            created_at,
            updated_at,
            last_heartbeat,
            completed_at
        FROM agent_sessions
        WHERE chat_id = $1
        "#,
        chat_id,
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(session)
}

/// Lists all active sessions for a workspace.
pub async fn get_active_sessions_by_workspace(
    conn: &mut DbConn,
    workspace_id: Uuid,
) -> Result<Vec<AgentSession>> {
    let sessions = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            id,
            workspace_id,
            chat_id,
            user_id,
            agent_type as "agent_type: AgentType",
            status as "status: SessionStatus",
            model,
            mode,
            current_task,
            created_at,
            updated_at,
            last_heartbeat,
            completed_at
        FROM agent_sessions
        WHERE workspace_id = $1
        AND status NOT IN ('completed', 'error')
        ORDER BY created_at DESC
        "#,
        workspace_id,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(sessions)
}

/// Lists all sessions for a user across all workspaces.
pub async fn get_sessions_by_user(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<AgentSession>> {
    let sessions = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            id,
            workspace_id,
            chat_id,
            user_id,
            agent_type as "agent_type: AgentType",
            status as "status: SessionStatus",
            model,
            mode,
            current_task,
            created_at,
            updated_at,
            last_heartbeat,
            completed_at
        FROM agent_sessions
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(sessions)
}

/// Lists all active sessions for a user.
pub async fn get_active_sessions_by_user(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<Vec<AgentSession>> {
    let sessions = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            id,
            workspace_id,
            chat_id,
            user_id,
            agent_type as "agent_type: AgentType",
            status as "status: SessionStatus",
            model,
            mode,
            current_task,
            created_at,
            updated_at,
            last_heartbeat,
            completed_at
        FROM agent_sessions
        WHERE user_id = $1
        AND status NOT IN ('completed', 'error')
        ORDER BY created_at DESC
        "#,
        user_id,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(sessions)
}

/// Updates the status of a session.
pub async fn update_session_status(
    conn: &mut DbConn,
    session_id: Uuid,
    status: SessionStatus,
) -> Result<AgentSession> {
    let updated_at = match status {
        SessionStatus::Completed | SessionStatus::Error => Some(Utc::now()),
        _ => None,
    };

    let session = sqlx::query_as!(
        AgentSession,
        r#"
        UPDATE agent_sessions
        SET status = $2, updated_at = NOW(), completed_at = $3
        WHERE id = $1
        RETURNING
            id,
            workspace_id,
            chat_id,
            user_id,
            agent_type as "agent_type: AgentType",
            status as "status: SessionStatus",
            model,
            mode,
            current_task,
            created_at,
            updated_at,
            last_heartbeat,
            completed_at
        "#,
        session_id,
        status as SessionStatus,
        updated_at,
    )
    .fetch_one(conn)
    .await
    .map_err(|e| {
        if e.to_string().to_lowercase().contains("no rows") {
            Error::NotFound(format!("Session with ID {} not found", session_id))
        } else {
            Error::Sqlx(e)
        }
    })?;

    Ok(session)
}

/// Updates the current task of a session.
pub async fn update_session_task(
    conn: &mut DbConn,
    session_id: Uuid,
    current_task: Option<String>,
) -> Result<AgentSession> {
    let session = sqlx::query_as!(
        AgentSession,
        r#"
        UPDATE agent_sessions
        SET current_task = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            workspace_id,
            chat_id,
            user_id,
            agent_type as "agent_type: AgentType",
            status as "status: SessionStatus",
            model,
            mode,
            current_task,
            created_at,
            updated_at,
            last_heartbeat,
            completed_at
        "#,
        session_id,
        current_task,
    )
    .fetch_one(conn)
    .await
    .map_err(|e| {
        if e.to_string().to_lowercase().contains("no rows") {
            Error::NotFound(format!("Session with ID {} not found", session_id))
        } else {
            Error::Sqlx(e)
        }
    })?;

    Ok(session)
}

/// Updates the heartbeat timestamp for a session (keeps it alive).
pub async fn update_heartbeat(conn: &mut DbConn, session_id: Uuid) -> Result<()> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE agent_sessions
        SET last_heartbeat = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    if rows_affected == 0 {
        return Err(Error::NotFound(format!(
            "Session with ID {} not found",
            session_id
        )));
    }

    Ok(())
}

/// Deletes a session by ID.
pub async fn delete_session(conn: &mut DbConn, session_id: Uuid) -> Result<()> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM agent_sessions
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    if rows_affected == 0 {
        return Err(Error::NotFound(format!(
            "Session with ID {} not found",
            session_id
        )));
    }

    Ok(())
}

/// Deletes all sessions for a specific chat.
pub async fn delete_session_by_chat(conn: &mut DbConn, chat_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM agent_sessions
        WHERE chat_id = $1
        "#,
    )
    .bind(chat_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}

/// Cleans up stale sessions that haven't sent a heartbeat recently.
pub async fn cleanup_stale_sessions(conn: &mut DbConn) -> Result<u64> {
    let threshold = Utc::now() - Duration::seconds(STALE_SESSION_THRESHOLD_SECONDS);

    let result = sqlx::query(
        r#"
        DELETE FROM agent_sessions
        WHERE last_heartbeat < $1
        AND status NOT IN ('completed', 'error')
        RETURNING id
        "#,
    )
    .bind(threshold)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.rows_affected())
}

/// Gets sessions that are considered stale (old heartbeat).
pub async fn get_stale_sessions(conn: &mut DbConn) -> Result<Vec<AgentSession>> {
    let threshold = Utc::now() - Duration::seconds(STALE_SESSION_THRESHOLD_SECONDS);

    let sessions = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            id,
            workspace_id,
            chat_id,
            user_id,
            agent_type as "agent_type: AgentType",
            status as "status: SessionStatus",
            model,
            mode,
            current_task,
            created_at,
            updated_at,
            last_heartbeat,
            completed_at
        FROM agent_sessions
        WHERE last_heartbeat < $1
        AND status NOT IN ('completed', 'error')
        ORDER BY last_heartbeat ASC
        "#,
        threshold,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(sessions)
}

/// Gets session count statistics for a workspace.
pub async fn get_workspace_session_stats(
    conn: &mut DbConn,
    workspace_id: Uuid,
) -> Result<WorkspaceSessionStats> {
    let stats = sqlx::query_as!(
        WorkspaceSessionStats,
        r#"
        SELECT
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE status = 'idle') as "idle: i64",
            COUNT(*) FILTER (WHERE status = 'running') as "running: i64",
            COUNT(*) FILTER (WHERE status = 'paused') as "paused: i64",
            COUNT(*) FILTER (WHERE status = 'completed') as "completed: i64",
            COUNT(*) FILTER (WHERE status = 'error') as "error: i64"
        FROM agent_sessions
        WHERE workspace_id = $1
        "#,
        workspace_id,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(stats)
}

/// Session statistics for a workspace
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceSessionStats {
    pub total: i64,
    pub idle: i64,
    pub running: i64,
    pub paused: i64,
    pub completed: i64,
    pub error: i64,
}
