use crate::{
    error::{Error, Result},
    models::agent_session::{AgentSession, AgentType, NewAgentSession, SessionStatus},
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::DbConn;

/// Default heartbeat timeout in seconds - sessions older than this are considered stale
pub const STALE_SESSION_THRESHOLD_SECONDS: i64 = 120; // 2 minutes

/// Gets or creates an agent session in the database.
///
/// This function implements intelligent session reuse:
/// 1. Creates a new session if none exists for the chat
/// 2. Reuses and updates an existing session if it's in a terminal state (completed, error, cancelled)
/// 3. Returns an error if an active session exists (idle, running, paused)
///
/// # Arguments
/// * `conn` - Database connection
/// * `new_session` - Session parameters to insert or use for update
///
/// # Returns
/// The created or updated agent session
///
/// # Errors
/// * `Conflict` - If an active session already exists for this chat
/// * `NotFound` - If workspace, chat, or user doesn't exist
pub async fn get_or_create_session(conn: &mut DbConn, new_session: NewAgentSession) -> Result<AgentSession> {
    tracing::debug!(
        chat_id = %new_session.chat_id,
        workspace_id = %new_session.workspace_id,
        user_id = %new_session.user_id,
        agent_type = %new_session.agent_type,
        model = %new_session.model,
        mode = %new_session.mode,
        "[AgentSessions] Get or create agent session"
    );

    // First, try to get an existing session for this chat
    let existing_session = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.chat_id = $1
        "#,
        new_session.chat_id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Error::Sqlx)?;

    match existing_session {
        Some(session) => {
            // Session exists - check if we can reuse it
            match session.status {
                SessionStatus::Completed | SessionStatus::Error | SessionStatus::Cancelled => {
                    // Terminal state - reuse by updating to idle
                    tracing::info!(
                        chat_id = %new_session.chat_id,
                        existing_status = %session.status,
                        session_id = %session.id,
                        "[AgentSessions] Reusing terminal session"
                    );

                    let updated = sqlx::query_as!(
                        AgentSession,
                        r#"
                        WITH updated AS (
                            UPDATE agent_sessions
                            SET
                                status = 'idle',
                                user_id = $2,
                                agent_type = $3,
                                model = $4,
                                mode = $5,
                                updated_at = NOW(),
                                last_heartbeat = NOW(),
                                completed_at = NULL,
                                error_message = NULL,
                                current_task = NULL
                            WHERE chat_id = $1
                            RETURNING *
                        )
                        SELECT
                            u.id,
                            u.workspace_id,
                            u.chat_id,
                            u.user_id,
                            u.agent_type as "agent_type: AgentType",
                            u.status as "status: SessionStatus",
                            u.model,
                            u.mode,
                            u.current_task,
                            u.error_message,
                            u.created_at,
                            u.updated_at,
                            u.last_heartbeat,
                            u.completed_at,
                            f.name as "chat_name?"
                        FROM updated u
                        LEFT JOIN files f ON u.chat_id = f.id
                        "#,
                        new_session.chat_id,
                        new_session.user_id,
                        new_session.agent_type as AgentType,
                        &new_session.model,
                        &new_session.mode
                    )
                    .fetch_one(conn)
                    .await
                    .map_err(Error::Sqlx)?;

                    tracing::info!(
                        session_id = %updated.id,
                        chat_id = %updated.chat_id,
                        "[AgentSessions] Reused session successfully"
                    );

                    Ok(updated)
                }
                _ => {
                    // Active session - return error
                    tracing::warn!(
                        chat_id = %new_session.chat_id,
                        existing_status = %session.status,
                        session_id = %session.id,
                        "[AgentSessions] Active session already exists for chat"
                    );
                    Err(Error::Conflict(format!(
                        "An active session already exists for this chat: {} (status: {}, session_id: {})",
                        new_session.chat_id, session.status, session.id
                    )))
                }
            }
        }
        None => {
            // No existing session - create a new one
            tracing::debug!(
                chat_id = %new_session.chat_id,
                "[AgentSessions] No existing session, creating new one"
            );
            create_session(conn, new_session).await
        }
    }
}

/// Creates a new agent session in the database.
pub async fn create_session(conn: &mut DbConn, new_session: NewAgentSession) -> Result<AgentSession> {
    tracing::debug!(
        chat_id = %new_session.chat_id,
        workspace_id = %new_session.workspace_id,
        user_id = %new_session.user_id,
        agent_type = %new_session.agent_type,
        model = %new_session.model,
        mode = %new_session.mode,
        "[AgentSessions] Creating new agent session"
    );

    // First insert the session
    let session = sqlx::query!(
        r#"
        INSERT INTO agent_sessions (
            workspace_id, chat_id, user_id, agent_type, status, model, mode
        )
        VALUES ($1, $2, $3, $4, 'idle', $5, $6)
        RETURNING id
        "#,
        new_session.workspace_id,
        new_session.chat_id,
        new_session.user_id,
        new_session.agent_type as AgentType,
        new_session.model,
        new_session.mode,
    )
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| {
        let error_msg = e.to_string().to_lowercase();

        // Check for unique constraint violations on chat_id
        if error_msg.contains("unique")
            || error_msg.contains("duplicate key")
            || error_msg.contains("agent_sessions_chat_id_key")
        {
            tracing::warn!(
                chat_id = %new_session.chat_id,
                "[AgentSessions] Session already exists for this chat",
            );
            Error::Conflict(format!(
                "An active session already exists for this chat: {}",
                new_session.chat_id
            ))
        } else {
            tracing::error!(
                chat_id = %new_session.chat_id,
                error = %e,
                "[AgentSessions] Failed to create session"
            );
            Error::Sqlx(e)
        }
    })?;

    // Then fetch with chat name
    let session = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.id = $1
        "#,
        session.id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    tracing::info!(
        session_id = %session.id,
        chat_id = %session.chat_id,
        workspace_id = %session.workspace_id,
        agent_type = %session.agent_type,
        model = %session.model,
        mode = %session.mode,
        status = %session.status,
        "[AgentSessions] Created new agent session"
    );

    Ok(session)
}

/// Gets a single session by its ID.
pub async fn get_session_by_id(conn: &mut DbConn, id: Uuid) -> Result<Option<AgentSession>> {
    tracing::trace!(session_id = %id, "[AgentSessions] Getting session by ID");

    let session = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.id = $1
        "#,
        id,
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    if session.is_some() {
        tracing::debug!(
            session_id = %id,
            found = session.is_some(),
            "[AgentSessions] Retrieved session by ID"
        );
    }

    Ok(session)
}

/// Gets a session by its chat ID (unique).
pub async fn get_session_by_chat(conn: &mut DbConn, chat_id: Uuid) -> Result<Option<AgentSession>> {
    tracing::trace!(chat_id = %chat_id, "[AgentSessions] Getting session by chat ID");

    let session = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.chat_id = $1
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
    tracing::trace!(
        workspace_id = %workspace_id,
        "[AgentSessions] Getting active sessions for workspace"
    );

    let sessions = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.workspace_id = $1
        AND s.status NOT IN ('completed', 'error', 'cancelled')
        ORDER BY s.created_at DESC
        "#,
        workspace_id,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    tracing::debug!(
        workspace_id = %workspace_id,
        count = sessions.len(),
        "[AgentSessions] Retrieved active sessions for workspace"
    );

    Ok(sessions)
}

/// Lists all sessions for a user across all workspaces.
pub async fn get_sessions_by_user(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<AgentSession>> {
    let sessions = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.user_id = $1
        ORDER BY s.created_at DESC
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
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.user_id = $1
        AND s.status NOT IN ('completed', 'error', 'cancelled')
        ORDER BY s.created_at DESC
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
    error_message: Option<String>,
) -> Result<AgentSession> {
    tracing::info!(
        session_id = %session_id,
        new_status = %status,
        error_message = ?error_message,
        "[AgentSessions] Query: Updating session status"
    );

    let completed_at = match status {
        SessionStatus::Completed | SessionStatus::Error => Some(Utc::now()),
        _ => None,
    };

    // Single query with CTE to UPDATE and fetch with chat_name in one round-trip
    let session = sqlx::query_as!(
        AgentSession,
        r#"
        WITH updated AS (
            UPDATE agent_sessions
            SET status = $2, updated_at = NOW(), completed_at = $3, error_message = $4
            WHERE id = $1
            RETURNING *
        )
        SELECT
            u.id,
            u.workspace_id,
            u.chat_id,
            u.user_id,
            u.agent_type as "agent_type: AgentType",
            u.status as "status: SessionStatus",
            u.model,
            u.mode,
            u.current_task,
            u.error_message,
            u.created_at,
            u.updated_at,
            u.last_heartbeat,
            u.completed_at,
            f.name as "chat_name?"
        FROM updated u
        LEFT JOIN files f ON u.chat_id = f.id
        "#,
        session_id,
        status as SessionStatus,
        completed_at,
        error_message.clone()
    )
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| {
        let error_str = e.to_string().to_lowercase();
        if error_str.contains("no rows") {
            tracing::error!(
                session_id = %session_id,
                "[AgentSessions] Failed to update status - session not found"
            );
            Error::NotFound(format!("Session with ID {} not found", session_id))
        } else {
            tracing::error!(
                session_id = %session_id,
                error = %e,
                "[AgentSessions] Failed to update session status"
            );
            Error::Sqlx(e)
        }
    })?;

    tracing::info!(
        session_id = %session.id,
        chat_id = %session.chat_id,
        workspace_id = %session.workspace_id,
        new_status = %session.status,
        error_message = ?session.error_message,
        completed_at = ?session.completed_at,
        "[AgentSessions] Updated session status successfully"
    );

    Ok(session)
}

/// Updates the current task of a session.
pub async fn update_session_task(
    conn: &mut DbConn,
    session_id: Uuid,
    current_task: Option<String>,
) -> Result<AgentSession> {
    tracing::debug!(
        session_id = %session_id,
        current_task = ?current_task,
        "[AgentSessions] Updating session task"
    );

    // First update the session
    sqlx::query(
        r#"
        UPDATE agent_sessions
        SET current_task = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .bind(current_task)
    .execute(&mut *conn)
    .await
    .map_err(|e| {
        if e.to_string().to_lowercase().contains("no rows") {
            tracing::error!(
                session_id = %session_id,
                "[AgentSessions] Failed to update task - session not found"
            );
            Error::NotFound(format!("Session with ID {} not found", session_id))
        } else {
            tracing::error!(
                session_id = %session_id,
                error = %e,
                "[AgentSessions] Failed to update session task"
            );
            Error::Sqlx(e)
        }
    })?;

    // Then fetch with chat name
    let session = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.id = $1
        "#,
        session_id
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

    tracing::debug!(
        session_id = %session.id,
        current_task = ?session.current_task,
        "[AgentSessions] Updated session task"
    );

    Ok(session)
}

/// Updates session metadata (model, mode, agent_type) with partial updates.
///
/// Only updates fields that are provided (Some values), leaving others unchanged.
pub async fn update_session_metadata(
    conn: &mut DbConn,
    session_id: Uuid,
    model: Option<String>,
    mode: Option<String>,
    agent_type: Option<AgentType>,
) -> Result<AgentSession> {
    tracing::debug!(
        session_id = %session_id,
        model = ?model,
        mode = ?mode,
        agent_type = ?agent_type,
        "[AgentSessions] Updating session metadata"
    );

    // Use a single atomic UPDATE with COALESCE to handle optional fields
    // This is more efficient than multiple separate UPDATE statements
    let model_sql = model.as_deref();
    let mode_sql = mode.as_deref();
    let agent_type_sql = agent_type.as_ref();

    sqlx::query(
        r#"
        UPDATE agent_sessions
        SET
            model = COALESCE($2, model),
            mode = COALESCE($3, mode),
            agent_type = COALESCE($4, agent_type),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .bind(model_sql)
    .bind(mode_sql)
    .bind(agent_type_sql)
    .execute(&mut *conn)
    .await
    .map_err(Error::Sqlx)?;

    // Fetch and return the updated session
    let session = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.id = $1
        "#,
        session_id
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(Error::Sqlx)?
    .ok_or_else(|| {
        tracing::error!(
            session_id = %session_id,
            "[AgentSessions] Failed to update metadata - session not found"
        );
        Error::NotFound(format!("Session with ID {} not found", session_id))
    })?;

    tracing::info!(
        session_id = %session.id,
        model = %session.model,
        mode = %session.mode,
        agent_type = %session.agent_type,
        "[AgentSessions] Updated session metadata"
    );

    Ok(session)
}

/// Updates the heartbeat timestamp for a session (keeps it alive).
pub async fn update_heartbeat(conn: &mut DbConn, session_id: Uuid) -> Result<()> {
    tracing::trace!(
        session_id = %session_id,
        "[AgentSessions] Updating heartbeat"
    );

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
        tracing::warn!(
            session_id = %session_id,
            "[AgentSessions] Failed to update heartbeat - session not found"
        );
        return Err(Error::NotFound(format!(
            "Session with ID {} not found",
            session_id
        )));
    }

    Ok(())
}

/// Deletes a session by ID.
pub async fn delete_session(conn: &mut DbConn, session_id: Uuid) -> Result<()> {
    tracing::debug!(
        session_id = %session_id,
        "[AgentSessions] Deleting session"
    );

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

    tracing::info!(
        session_id = %session_id,
        "[AgentSessions] Deleted session"
    );

    Ok(())
}

/// Deletes all sessions for a specific chat.
pub async fn delete_session_by_chat(conn: &mut DbConn, chat_id: Uuid) -> Result<()> {
    tracing::debug!(
        chat_id = %chat_id,
        "[AgentSessions] Deleting all sessions for chat"
    );

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
        AND status NOT IN ('completed', 'error', 'cancelled')
        RETURNING id
        "#,
    )
    .bind(threshold)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    let count = result.rows_affected();

    if count > 0 {
        tracing::info!(
            count,
            threshold_seconds = STALE_SESSION_THRESHOLD_SECONDS,
            "[AgentSessions] Cleaned up stale sessions"
        );
    }

    Ok(count)
}

/// Gets sessions that are considered stale (old heartbeat).
pub async fn get_stale_sessions(conn: &mut DbConn) -> Result<Vec<AgentSession>> {
    let threshold = Utc::now() - Duration::seconds(STALE_SESSION_THRESHOLD_SECONDS);

    let sessions = sqlx::query_as!(
        AgentSession,
        r#"
        SELECT
            s.id,
            s.workspace_id,
            s.chat_id,
            s.user_id,
            s.agent_type as "agent_type: AgentType",
            s.status as "status: SessionStatus",
            s.model,
            s.mode,
            s.current_task,
            s.error_message,
            s.created_at,
            s.updated_at,
            s.last_heartbeat,
            s.completed_at,
            f.name as "chat_name?"
        FROM agent_sessions s
        LEFT JOIN files f ON s.chat_id = f.id
        WHERE s.last_heartbeat < $1
        AND s.status NOT IN ('completed', 'error', 'cancelled')
        ORDER BY s.last_heartbeat ASC
        "#,
        threshold,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    if !sessions.is_empty() {
        tracing::warn!(
            count = sessions.len(),
            "[AgentSessions] Found stale sessions"
        );
    }

    Ok(sessions)
}

/// Gets session count statistics for a workspace.
pub async fn get_workspace_session_stats(
    conn: &mut DbConn,
    workspace_id: Uuid,
) -> Result<WorkspaceSessionStats> {
    // Use separate counts with CASE instead of FILTER to avoid SQLx type issues
    let row = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as "total!: i64",
            SUM(CASE WHEN status = 'idle' THEN 1 ELSE 0 END) as "idle!: i64",
            SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END) as "running!: i64",
            SUM(CASE WHEN status = 'paused' THEN 1 ELSE 0 END) as "paused!: i64",
            SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as "completed!: i64",
            SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as "error!: i64"
        FROM agent_sessions
        WHERE workspace_id = $1
        "#,
        workspace_id,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(WorkspaceSessionStats {
        total: row.total,
        idle: row.idle,
        running: row.running,
        paused: row.paused,
        completed: row.completed,
        error: row.error,
    })
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
