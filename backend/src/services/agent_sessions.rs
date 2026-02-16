//! Agent Session Service
//!
//! This module provides business logic for managing AI agent sessions.
//! It handles session creation, status updates, heartbeat management,
//! and coordinates with the ChatActor for pause/resume/cancel operations.

use crate::{
    error::{Error, Result},
    models::agent_session::{
        AgentSession, AgentSessionInfo, AgentSessionsListResponse, NewAgentSession,
        PauseSessionRequest, SessionActionResponse, SessionStatus,
    },
    queries::agent_sessions,
    DbConn,
};
use uuid::Uuid;

// ============================================================================
// SESSION CREATION
// ============================================================================

/// Creates a new agent session for a chat.
///
/// # Arguments
/// * `conn` - Database connection
/// * `workspace_id` - Workspace ID
/// * `chat_id` - Chat file ID (unique per session)
/// * `user_id` - User who initiated the session
/// * `agent_type` - Type of agent (assistant, planner, builder)
/// * `model` - AI model name
/// * `mode` - Operating mode (chat, plan, build)
///
/// # Returns
/// The created agent session
///
/// # Errors
/// * `Conflict` - If a session already exists for this chat
/// * `NotFound` - If workspace, chat, or user doesn't exist
pub async fn create_session(
    conn: &mut DbConn,
    workspace_id: Uuid,
    chat_id: Uuid,
    user_id: Uuid,
    agent_type: crate::models::agent_session::AgentType,
    model: String,
    mode: String,
) -> Result<AgentSession> {
    tracing::info!(
        workspace_id = %workspace_id,
        chat_id = %chat_id,
        user_id = %user_id,
        agent_type = %agent_type,
        model = %model,
        mode = %mode,
        "[AgentSessions] Service: Creating new agent session"
    );

    // Validate inputs
    validate_model_name(&model)?;
    validate_mode(&mode)?;

    let new_session = NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type,
        model,
        mode,
    };

    agent_sessions::create_session(conn, new_session).await
}

// ============================================================================
// SESSION RETRIEVAL
// ============================================================================

/// Gets a session by ID with authorization check.
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - Session ID
/// * `user_id` - User requesting the session (for authorization)
///
/// # Returns
/// The session if found and user has access
///
/// # Errors
/// * `NotFound` - If session doesn't exist or user doesn't have access
pub async fn get_session(
    conn: &mut DbConn,
    session_id: Uuid,
    user_id: Uuid,
) -> Result<AgentSession> {
    tracing::debug!(
        session_id = %session_id,
        user_id = %user_id,
        "[AgentSessions] Service: Getting session with authorization check"
    );

    let session = agent_sessions::get_session_by_id(conn, session_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Session {} not found", session_id)))?;

    // Authorization check: user must be the session owner
    if session.user_id != user_id {
        tracing::warn!(
            session_id = %session_id,
            user_id = %user_id,
            session_owner = %session.user_id,
            "[AgentSessions] Service: Authorization failed - user does not own this session"
        );
        return Err(Error::Forbidden(
            "You don't have permission to access this session".to_string(),
        ));
    }

    Ok(session)
}

/// Lists all active sessions for a workspace.
///
/// # Arguments
/// * `conn` - Database connection
/// * `workspace_id` - Workspace ID
/// * `user_id` - User requesting the sessions (for authorization)
///
/// # Returns
/// List of active sessions for the workspace
///
/// # Errors
/// * `Forbidden` - If user doesn't have access to the workspace
pub async fn list_workspace_sessions(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<AgentSessionsListResponse> {
    tracing::debug!(
        workspace_id = %workspace_id,
        user_id = %user_id,
        "[AgentSessions] Service: Listing active sessions for workspace"
    );

    // TODO: Add workspace membership check when workspace access control is available
    // For now, we'll just get all active sessions

    let sessions =
        agent_sessions::get_active_sessions_by_workspace(conn, workspace_id).await?;

    let total = sessions.len();
    let session_infos: Vec<AgentSessionInfo> =
        sessions.into_iter().map(Into::into).collect();

    tracing::debug!(
        workspace_id = %workspace_id,
        total_sessions = total,
        "[AgentSessions] Service: Retrieved active sessions for workspace"
    );

    Ok(AgentSessionsListResponse {
        sessions: session_infos,
        total,
    })
}

/// Lists all active sessions for a user.
///
/// # Arguments
/// * `conn` - Database connection
/// * `user_id` - User ID
///
/// # Returns
/// List of active sessions for the user
pub async fn list_user_sessions(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<AgentSessionsListResponse> {
    tracing::debug!(
        user_id = %user_id,
        "[AgentSessions] Service: Listing active sessions for user"
    );

    let sessions = agent_sessions::get_active_sessions_by_user(conn, user_id).await?;

    let total = sessions.len();
    let session_infos: Vec<AgentSessionInfo> =
        sessions.into_iter().map(Into::into).collect();

    Ok(AgentSessionsListResponse {
        sessions: session_infos,
        total,
    })
}

// ============================================================================
// SESSION STATUS UPDATES
// ============================================================================

/// Updates the status of a session.
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - Session ID
/// * `status` - New status
/// * `user_id` - User requesting the update (for authorization)
///
/// # Returns
/// The updated session
///
/// # Errors
/// * `NotFound` - If session doesn't exist or user doesn't have access
/// * `Forbidden` - If user doesn't have permission to update this session
pub async fn update_session_status(
    conn: &mut DbConn,
    session_id: Uuid,
    status: SessionStatus,
    user_id: Uuid,
) -> Result<AgentSession> {
    tracing::info!(
        session_id = %session_id,
        new_status = %status,
        user_id = %user_id,
        "[AgentSessions] Service: Updating session status"
    );

    // Verify ownership first
    let session = get_session(conn, session_id, user_id).await?;

    tracing::debug!(
        session_id = %session_id,
        old_status = %session.status,
        new_status = %status,
        "[AgentSessions] Service: Status transition"
    );

    // Validate status transition
    validate_status_transition(session.status, status)?;

    agent_sessions::update_session_status(conn, session_id, status).await
}

/// Updates the current task of a session.
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - Session ID
/// * `current_task` - Description of current task (or None to clear)
/// * `user_id` - User requesting the update (for authorization)
///
/// # Returns
/// The updated session
pub async fn update_session_task(
    conn: &mut DbConn,
    session_id: Uuid,
    current_task: Option<String>,
    user_id: Uuid,
) -> Result<AgentSession> {
    tracing::debug!(
        session_id = %session_id,
        current_task = ?current_task,
        user_id = %user_id,
        "[AgentSessions] Service: Updating session task"
    );

    // Verify ownership first
    get_session(conn, session_id, user_id).await?;

    agent_sessions::update_session_task(conn, session_id, current_task).await
}

/// Updates session metadata (model, mode, agent_type).
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - Session ID
/// * `model` - New model name (None = no change)
/// * `mode` - New mode (None = no change)
/// * `agent_type` - New agent type (None = no change)
/// * `user_id` - User requesting the update (for authorization)
///
/// # Returns
/// The updated session
///
/// # Errors
/// * `NotFound` - If session doesn't exist or user doesn't have access
/// * `Forbidden` - If user doesn't have permission to update this session
pub async fn update_session_metadata(
    conn: &mut DbConn,
    session_id: Uuid,
    model: Option<String>,
    mode: Option<String>,
    agent_type: Option<crate::models::agent_session::AgentType>,
    user_id: Uuid,
) -> Result<AgentSession> {
    tracing::debug!(
        session_id = %session_id,
        model = ?model,
        mode = ?mode,
        agent_type = ?agent_type,
        user_id = %user_id,
        "[AgentSessions] Service: Updating session metadata"
    );

    // Validate inputs if provided
    if let Some(ref m) = model {
        validate_model_name(m)?;
    }
    if let Some(ref m) = mode {
        validate_mode(m)?;
    }

    // Verify ownership first
    get_session(conn, session_id, user_id).await?;

    agent_sessions::update_session_metadata(conn, session_id, model, mode, agent_type).await
}

/// Updates the heartbeat timestamp for a session (keeps it alive).
///
/// This should be called periodically (e.g., every 30 seconds) while
/// the agent is actively processing.
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - Session ID
///
/// # Errors
/// * `NotFound` - If session doesn't exist
pub async fn update_heartbeat(conn: &mut DbConn, session_id: Uuid) -> Result<()> {
    // Heartbeat updates are very frequent, so use trace level
    tracing::trace!(
        session_id = %session_id,
        "[AgentSessions] Service: Updating heartbeat"
    );

    agent_sessions::update_heartbeat(conn, session_id).await
}

// ============================================================================
// SESSION ACTIONS
// ============================================================================

/// Pauses an active session.
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - Session ID
/// * `request` - Pause request with optional reason
/// * `user_id` - User requesting the pause (for authorization)
///
/// # Returns
/// Action response with updated session
///
/// # Errors
/// * `NotFound` - If session doesn't exist or user doesn't have access
/// * `Forbidden` - If user doesn't have permission to pause this session
/// * `Conflict` - If session is not in a pausable state
pub async fn pause_session(
    conn: &mut DbConn,
    session_id: Uuid,
    _request: PauseSessionRequest,
    user_id: Uuid,
) -> Result<SessionActionResponse> {
    tracing::info!(
        session_id = %session_id,
        user_id = %user_id,
        "[AgentSessions] Service: Pausing session"
    );

    let session = get_session(conn, session_id, user_id).await?;

    // Check if session can be paused
    match session.status {
        SessionStatus::Running | SessionStatus::Idle => {
            tracing::debug!(
                session_id = %session_id,
                current_status = %session.status,
                "[AgentSessions] Service: Session can be paused"
            );
        }
        SessionStatus::Paused => {
            tracing::warn!(
                session_id = %session_id,
                "[AgentSessions] Service: Session is already paused"
            );
            return Err(Error::Conflict(
                "Session is already paused".to_string(),
            ));
        }
        SessionStatus::Completed | SessionStatus::Error => {
            tracing::warn!(
                session_id = %session_id,
                status = %session.status,
                "[AgentSessions] Service: Cannot pause session - terminal state"
            );
            return Err(Error::Conflict(format!(
                "Cannot pause session with status: {}",
                session.status
            )));
        }
    }

    // TODO: Coordinate with ChatActor to actually pause the agent processing
    // For now, we'll just update the status
    tracing::debug!(
        session_id = %session_id,
        "[AgentSessions] Service: Updating session status to paused"
    );

    let updated_session =
        agent_sessions::update_session_status(conn, session_id, SessionStatus::Paused).await?;

    tracing::info!(
        session_id = %session_id,
        "[AgentSessions] Service: Successfully paused session"
    );

    Ok(SessionActionResponse {
        session: updated_session.into(),
        message: "Session paused successfully".to_string(),
    })
}

/// Resumes a paused session.
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - Session ID
/// * `task` - Optional task to resume with
/// * `user_id` - User requesting the resume (for authorization)
///
/// # Returns
/// Action response with updated session
///
/// # Errors
/// * `NotFound` - If session doesn't exist or user doesn't have access
/// * `Forbidden` - If user doesn't have permission to resume this session
/// * `Conflict` - If session is not paused
pub async fn resume_session(
    conn: &mut DbConn,
    session_id: Uuid,
    task: Option<String>,
    user_id: Uuid,
) -> Result<SessionActionResponse> {
    tracing::info!(
        session_id = %session_id,
        task = ?task,
        user_id = %user_id,
        "[AgentSessions] Service: Resuming session"
    );

    let session = get_session(conn, session_id, user_id).await?;

    // Check if session is paused
    if session.status != SessionStatus::Paused {
        tracing::warn!(
            session_id = %session_id,
            status = %session.status,
            "[AgentSessions] Service: Cannot resume - not paused"
        );
        return Err(Error::Conflict(format!(
            "Cannot resume session with status: {}. Only paused sessions can be resumed.",
            session.status
        )));
    }

    // TODO: Coordinate with ChatActor to resume agent processing
    // For now, we'll just update the status to idle
    tracing::debug!(
        session_id = %session_id,
        "[AgentSessions] Service: Updating session status to idle"
    );

    let updated_session =
        agent_sessions::update_session_status(conn, session_id, SessionStatus::Idle).await?;

    // If task provided, update it
    let updated_session = if let Some(task) = task {
        tracing::debug!(
            session_id = %session_id,
            task = %task,
            "[AgentSessions] Service: Updating session task"
        );
        agent_sessions::update_session_task(conn, session_id, Some(task)).await?
    } else {
        updated_session
    };

    tracing::info!(
        session_id = %session_id,
        "[AgentSessions] Service: Successfully resumed session"
    );

    Ok(SessionActionResponse {
        session: updated_session.into(),
        message: "Session resumed successfully".to_string(),
    })
}

/// Cancels/stops an active session.
///
/// This deletes the session from the database. When the user chats again,
/// a new session will be created with full chat history preserved.
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - Session ID
/// * `user_id` - User requesting the cancellation (for authorization)
///
/// # Returns
/// Action response confirming cancellation
///
/// # Errors
/// * `NotFound` - If session doesn't exist or user doesn't have access
/// * `Forbidden` - If user doesn't have permission to cancel this session
pub async fn cancel_session(
    conn: &mut DbConn,
    session_id: Uuid,
    user_id: Uuid,
) -> Result<SessionActionResponse> {
    tracing::info!(
        session_id = %session_id,
        user_id = %user_id,
        "[AgentSessions] Service: Cancelling session"
    );

    let session = get_session(conn, session_id, user_id).await?;

    // Check if session can be cancelled
    match session.status {
        SessionStatus::Completed | SessionStatus::Error => {
            tracing::warn!(
                session_id = %session_id,
                status = %session.status,
                "[AgentSessions] Service: Cannot cancel - terminal state"
            );
            return Err(Error::Conflict(format!(
                "Cannot cancel session with status: {}",
                session.status
            )));
        }
        _ => {
            tracing::debug!(
                session_id = %session_id,
                status = %session.status,
                "[AgentSessions] Service: Session can be cancelled"
            );
        }
    }

    // Delete the session instead of marking as completed
    // This allows a new session to be created when the user chats again
    // Chat history is preserved in the chat_messages table
    tracing::debug!(
        session_id = %session_id,
        chat_id = %session.chat_id,
        "[AgentSessions] Service: Deleting session on cancel"
    );

    agent_sessions::delete_session_by_chat(conn, session.chat_id).await?;

    tracing::info!(
        session_id = %session_id,
        chat_id = %session.chat_id,
        "[AgentSessions] Service: Successfully cancelled and deleted session"
    );

    Ok(SessionActionResponse {
        session: session.into(), // Return last known state
        message: "Session cancelled successfully".to_string(),
    })
}

/// Deletes a session.
///
/// # Arguments
/// * `conn` - Database connection
/// * `session_id` - Session ID
/// * `user_id` - User requesting the deletion (for authorization)
///
/// # Errors
/// * `NotFound` - If session doesn't exist or user doesn't have access
/// * `Forbidden` - If user doesn't have permission to delete this session
pub async fn delete_session(
    conn: &mut DbConn,
    session_id: Uuid,
    user_id: Uuid,
) -> Result<()> {
    tracing::info!(
        session_id = %session_id,
        user_id = %user_id,
        "[AgentSessions] Service: Deleting session"
    );

    // Verify ownership first
    get_session(conn, session_id, user_id).await?;

    agent_sessions::delete_session(conn, session_id).await
}

// ============================================================================
// SESSION CLEANUP
// ============================================================================

/// Cleans up stale sessions that haven't sent a heartbeat recently.
///
/// This should be called periodically (e.g., every minute) by a background job.
///
/// # Arguments
/// * `conn` - Database connection
///
/// # Returns
/// Number of sessions cleaned up
pub async fn cleanup_stale_sessions(conn: &mut DbConn) -> Result<u64> {
    tracing::debug!(
        "[AgentSessions] Service: Checking for stale sessions"
    );

    let count = agent_sessions::cleanup_stale_sessions(conn).await?;

    if count > 0 {
        tracing::info!(
            count,
            "[AgentSessions] Service: Cleaned up stale sessions"
        );
    }

    Ok(count)
}

// ============================================================================
// VALIDATION FUNCTIONS
// ============================================================================

/// Validates that a model name is not empty.
fn validate_model_name(model: &str) -> Result<()> {
    if model.trim().is_empty() {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "model".to_string(),
            message: "Model name cannot be empty".to_string(),
        }));
    }

    // Could add more validation here (e.g., check against allowed models)

    Ok(())
}

/// Validates that a mode is one of the allowed values.
fn validate_mode(mode: &str) -> Result<()> {
    let valid_modes = ["chat", "plan", "build"];

    if !valid_modes.contains(&mode.to_lowercase().as_str()) {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "mode".to_string(),
            message: format!("Mode must be one of: {}", valid_modes.join(", ")),
        }));
    }

    Ok(())
}

/// Validates that a status transition is allowed.
fn validate_status_transition(
    current_status: SessionStatus,
    new_status: SessionStatus,
) -> Result<()> {
    tracing::trace!(
        current_status = %current_status,
        new_status = %new_status,
        "[AgentSessions] Service: Validating status transition"
    );

    match (current_status, new_status) {
        // Completed and Error are terminal states - cannot transition
        (SessionStatus::Completed | SessionStatus::Error, _) => {
            tracing::warn!(
                current_status = %current_status,
                new_status = %new_status,
                "[AgentSessions] Service: Invalid transition - terminal state"
            );
            Err(Error::Conflict(format!(
                "Cannot transition from {} to {}",
                current_status, new_status
            )))
        }

        // Can always transition to running from any non-terminal state
        (_, SessionStatus::Running) => Ok(()),

        // Running can transition to idle, paused, completed, error
        (SessionStatus::Running, SessionStatus::Idle) => Ok(()),
        (SessionStatus::Running, SessionStatus::Paused) => Ok(()),
        (SessionStatus::Running, SessionStatus::Completed) => Ok(()),
        (SessionStatus::Running, SessionStatus::Error) => Ok(()),

        // Idle can transition to paused
        (SessionStatus::Idle, SessionStatus::Paused) => Ok(()),

        // Paused can transition to idle or completed
        (SessionStatus::Paused, SessionStatus::Idle) => Ok(()),
        (SessionStatus::Paused, SessionStatus::Completed) => Ok(()),

        // All other transitions are invalid
        (from, to) => {
            tracing::warn!(
                from = %from,
                to = %to,
                "[AgentSessions] Service: Invalid status transition"
            );
            Err(Error::Conflict(format!(
                "Invalid status transition from {} to {}",
                from, to
            )))
        }
    }
}
