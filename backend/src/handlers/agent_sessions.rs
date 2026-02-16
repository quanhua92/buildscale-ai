//! Agent Session Management handlers
//!
//! This module provides HTTP handlers for agent session operations.
//! Handlers follow the thin-layer pattern: they validate inputs, delegate to services,
//! and return responses. All business logic is in the service layer.

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use uuid::Uuid;
use crate::{
    error::Result,
    middleware::auth::AuthenticatedUser,
    middleware::workspace_access::WorkspaceAccess,
    models::agent_session::{PauseSessionRequest, SessionActionResponse},
    services::agent_sessions,
    state::AppState,
};

// ============================================================================
// LIST WORKSPACE SESSIONS
// ============================================================================

/// GET /api/v1/workspaces/:workspace_id/agent-sessions
///
/// Lists all active agent sessions in a workspace.
/// Requires workspace membership.
///
/// # Parameters
/// - `workspace_id`: Workspace UUID
///
/// # Headers
/// - Authorization: Bearer <access_token>
///
/// # Returns
/// JSON response containing active sessions list.
///
/// # HTTP Status Codes
/// - `200 OK`: Sessions retrieved successfully
/// - `403 FORBIDDEN`: Insufficient permissions
/// - `404 NOT_FOUND`: Workspace not found
pub async fn list_workspace_sessions(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        operation = "list_workspace_sessions",
        workspace_id = %workspace_id,
        requester_id = %auth_user.id,
        "Listing workspace agent sessions",
    );

    let mut conn = acquire_db_connection(&state, "list_workspace_sessions").await?;

    let response = agent_sessions::list_workspace_sessions(&mut conn, workspace_id, auth_user.id)
        .await
        .inspect_err(|e| log_handler_error("list_workspace_sessions", e))?;

    tracing::info!(
        operation = "list_workspace_sessions",
        workspace_id = %workspace_id,
        count = response.total,
        "Sessions listed successfully",
    );

    Ok(Json(serde_json::json!({
        "sessions": response.sessions,
        "total": response.total,
    })))
}

// ============================================================================
// GET SESSION DETAILS
// ============================================================================

/// GET /api/v1/agent-sessions/:id
///
/// Gets details of a specific agent session.
/// Requires being the session owner.
///
/// # Parameters
/// - `id`: Session UUID
///
/// # Headers
/// - Authorization: Bearer <access_token>
///
/// # Returns
/// JSON response containing session details.
///
/// # HTTP Status Codes
/// - `200 OK`: Session retrieved successfully
/// - `403 FORBIDDEN`: Not the session owner
/// - `404 NOT_FOUND`: Session not found
pub async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        operation = "get_session",
        session_id = %session_id,
        requester_id = %auth_user.id,
        "Getting agent session details",
    );

    let mut conn = acquire_db_connection(&state, "get_session").await?;

    let session = agent_sessions::get_session(&mut conn, session_id, auth_user.id)
        .await
        .inspect_err(|e| log_handler_error("get_session", e))?;

    tracing::info!(
        operation = "get_session",
        session_id = %session_id,
        user_id = %auth_user.id,
        status = %session.status,
        "Session retrieved successfully",
    );

    Ok(Json(serde_json::json!({
        "session": session,
    })))
}

// ============================================================================
// PAUSE SESSION
// ============================================================================

/// POST /api/v1/agent-sessions/:id/pause
///
/// Pauses an active agent session.
/// Requires being the session owner.
///
/// # Parameters
/// - `id`: Session UUID
///
/// # Headers
/// - Authorization: Bearer <access_token>
/// - Content-Type: application/json
///
/// # Request Body
/// - `reason`: Optional reason for pausing
///
/// # Returns
/// JSON response confirming pause.
///
/// # HTTP Status Codes
/// - `200 OK`: Session paused successfully
/// - `403 FORBIDDEN`: Not the session owner
/// - `404 NOT_FOUND`: Session not found
/// - `409 CONFLICT`: Session cannot be paused
pub async fn pause_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<PauseSessionRequest>,
) -> Result<Json<SessionActionResponse>> {
    tracing::info!(
        operation = "pause_session",
        session_id = %session_id,
        requester_id = %auth_user.id,
        reason = ?request.reason,
        "Pausing agent session",
    );

    let mut conn = acquire_db_connection(&state, "pause_session").await?;

    // Get the session first to find the chat_id
    let session = agent_sessions::get_session(&mut conn, session_id, auth_user.id)
        .await
        .inspect_err(|e| log_handler_error("pause_session", e))?;

    // Send Pause command to the ChatActor to pause any active interaction
    if let Some(handle) = state.agents.active_agents.read_async(&session.chat_id, |_, h| h.clone()).await {
        tracing::info!(
            operation = "pause_session",
            chat_id = %session.chat_id,
            "Found active ChatActor, sending Pause command"
        );

        // Send the Pause command to the actor
        let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
        let pause_cmd = crate::services::chat::registry::AgentCommand::Pause {
            reason: request.reason.clone(),
            responder: std::sync::Arc::new(tokio::sync::Mutex::new(Some(responder_tx))),
        };

        if let Err(_) = handle.command_tx.send(pause_cmd).await {
            tracing::warn!(
                operation = "pause_session",
                chat_id = %session.chat_id,
                "Failed to send Pause command to ChatActor (channel closed)"
            );
        } else {
            // Wait for the actor to acknowledgment
            let _ = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                responder_rx
            ).await;
        }
    } else {
        tracing::debug!(
            operation = "pause_session",
            chat_id = %session.chat_id,
            "No active ChatActor found for this chat"
        );
    }

    // Now update the session status in the database
    let response = agent_sessions::pause_session(&mut conn, session_id, request, auth_user.id)
        .await
        .inspect_err(|e| log_handler_error("pause_session", e))?;

    tracing::info!(
        operation = "pause_session",
        session_id = %session_id,
        user_id = %auth_user.id,
        "Session paused successfully",
    );

    Ok(Json(response))
}

// ============================================================================
// RESUME SESSION
// ============================================================================

/// POST /api/v1/agent-sessions/:id/resume
///
/// Resumes a paused agent session.
/// Requires being the session owner.
///
/// # Parameters
/// - `id`: Session UUID
///
/// # Headers
/// - Authorization: Bearer <access_token>
/// - Content-Type: application/json
///
/// # Request Body
/// - `task`: Optional task to resume with
///
/// # Returns
/// JSON response confirming resume.
///
/// # HTTP Status Codes
/// - `200 OK`: Session resumed successfully
/// - `403 FORBIDDEN`: Not the session owner
/// - `404 NOT_FOUND`: Session not found
/// - `409 CONFLICT`: Session cannot be resumed
pub async fn resume_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<crate::models::agent_session::ResumeSessionRequest>,
) -> Result<Json<SessionActionResponse>> {
    tracing::info!(
        operation = "resume_session",
        session_id = %session_id,
        requester_id = %auth_user.id,
        task = ?request.task,
        "Resuming agent session",
    );

    let mut conn = acquire_db_connection(&state, "resume_session").await?;

    let response = agent_sessions::resume_session(
        &mut conn,
        session_id,
        request.task,
        auth_user.id,
    )
    .await
    .inspect_err(|e| log_handler_error("resume_session", e))?;

    tracing::info!(
        operation = "resume_session",
        session_id = %session_id,
        user_id = %auth_user.id,
        "Session resumed successfully",
    );

    Ok(Json(response))
}

// ============================================================================
// CANCEL SESSION
// ============================================================================

/// DELETE /api/v1/agent-sessions/:id
///
/// Cancels/stops an active agent session.
/// Requires being the session owner.
///
/// # Parameters
/// - `id`: Session UUID
///
/// # Headers
/// - Authorization: Bearer <access_token>
///
/// # Returns
/// JSON response confirming cancellation.
///
/// # HTTP Status Codes
/// - `200 OK`: Session cancelled successfully
/// - `403 FORBIDDEN`: Not the session owner
/// - `404 NOT_FOUND`: Session not found
/// - `409 CONFLICT`: Session cannot be cancelled
pub async fn cancel_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<SessionActionResponse>> {
    tracing::info!(
        operation = "cancel_session",
        session_id = %session_id,
        requester_id = %auth_user.id,
        "Cancelling agent session",
    );

    let mut conn = acquire_db_connection(&state, "cancel_session").await?;

    // Get the session first to find the chat_id
    let session = agent_sessions::get_session(&mut conn, session_id, auth_user.id)
        .await
        .inspect_err(|e| log_handler_error("cancel_session", e))?;

    // Send Cancel command to the ChatActor to stop it gracefully
    if let Some(handle) = state.agents.active_agents.read_async(&session.chat_id, |_, h| h.clone()).await {
        tracing::info!(
            operation = "cancel_session",
            chat_id = %session.chat_id,
            "Found active ChatActor, sending Cancel command"
        );

        // Send the Cancel command to the actor
        let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
        let cancel_cmd = crate::services::chat::registry::AgentCommand::Cancel {
            reason: "Session cancelled by user".to_string(),
            responder: std::sync::Arc::new(tokio::sync::Mutex::new(Some(responder_tx))),
        };

        if let Err(_) = handle.command_tx.send(cancel_cmd).await {
            tracing::warn!(
                operation = "cancel_session",
                chat_id = %session.chat_id,
                "Failed to send Cancel command to ChatActor (channel closed)"
            );
        } else {
            // Wait for the actor to acknowledge cancellation
            let _ = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                responder_rx
            ).await;

            // Remove the actor from the registry
            state.agents.remove(&session.chat_id).await;
        }
    } else {
        tracing::debug!(
            operation = "cancel_session",
            chat_id = %session.chat_id,
            "No active ChatActor found for this chat"
        );
    }

    // Also cancel any active stream cancellation tokens
    state.agents.cancel_stream(&session.chat_id).await;

    // Now delete the session from the database
    let response = agent_sessions::cancel_session(&mut conn, session_id, auth_user.id)
        .await
        .inspect_err(|e| log_handler_error("cancel_session", e))?;

    tracing::info!(
        operation = "cancel_session",
        session_id = %session_id,
        user_id = %auth_user.id,
        "Session cancelled successfully",
    );

    Ok(Json(response))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Helper to log handler errors with appropriate level
fn log_handler_error(operation: &str, e: &crate::error::Error) {
    match e {
        crate::error::Error::Validation(_)
        | crate::error::Error::NotFound(_)
        | crate::error::Error::Forbidden(_)
        | crate::error::Error::Conflict(_) => {
            tracing::warn!(operation = operation, error = %e, "Handler operation failed");
        }
        _ => {
            tracing::error!(operation = operation, error = %e, "Handler operation failed");
        }
    }
}

/// Helper to acquire database connection with consistent error logging
async fn acquire_db_connection(
    state: &AppState,
    operation: &'static str,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>> {
    state.pool.acquire().await.map_err(|e| {
        tracing::error!(
            operation = operation,
            error_code = "DATABASE_ACQUISITION_FAILED",
            error = %e,
            "Failed to acquire database connection",
        );
        crate::error::Error::Internal(format!(
            "Failed to acquire database connection: {}",
            e
        ))
    })
}
