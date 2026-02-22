//! Session management methods for ChatActor
//!
//! Helper functions for creating and managing agent sessions,
//! including database tracking and heartbeat tasks.

use crate::models::agent_session::AgentType;
use crate::models::chat::DEFAULT_CHAT_MODEL;
use crate::queries;
use crate::services::agent_sessions;
use crate::DbPool;
use crate::error::Result;
use tokio::task::JoinHandle;
use uuid::Uuid;

/// Creates a new agent session in the database.
pub async fn create_session(
    pool: &DbPool,
    workspace_id: Uuid,
    chat_id: Uuid,
    user_id: Uuid,
    default_persona: &str,
) -> Result<Uuid> {
    tracing::info!(
        chat_id = %chat_id,
        workspace_id = %workspace_id,
        user_id = %user_id,
        persona = %default_persona,
        "[ChatActor] Creating agent session in database"
    );

    let mut conn = pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

    // Get the chat file's latest version to extract actual model and mode
    // This ensures the session is created with the correct values from the chat config
    let (actual_model, actual_mode) = match queries::files::get_latest_version(&mut conn, chat_id).await {
        Ok(version) => {
            // Extract model and mode from app_data
            let model = version.app_data.get("model")
                .and_then(|v| v.as_str())
                .unwrap_or(DEFAULT_CHAT_MODEL)
                .to_string();

            let mode = version.app_data.get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("plan")
                .to_string();

            tracing::debug!(
                chat_id = %chat_id,
                model = %model,
                mode = %mode,
                "[ChatActor] Extracted model and mode from chat file app_data"
            );

            (model, mode)
        }
        Err(e) => {
            tracing::warn!(
                chat_id = %chat_id,
                error = %e,
                "[ChatActor] Failed to get chat file version, using defaults"
            );
            (DEFAULT_CHAT_MODEL.to_string(), "plan".to_string())
        }
    };

    // Determine agent type from mode (not from persona)
    let agent_type = match actual_mode.as_str() {
        "plan" => AgentType::Planner,
        "build" => AgentType::Builder,
        _ => AgentType::Assistant,
    };

    tracing::debug!(
        chat_id = %chat_id,
        agent_type = %agent_type,
        model = %actual_model,
        mode = %actual_mode,
        "[ChatActor] Session configuration determined from chat file"
    );

    let session = agent_sessions::get_or_create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        agent_type,
        actual_model,
        actual_mode,
    )
    .await?;

    tracing::info!(
        chat_id = %chat_id,
        session_id = %session.id,
        model = %session.model,
        mode = %session.mode,
        agent_type = %session.agent_type,
        "[ChatActor] Successfully created agent session with correct values"
    );

    Ok(session.id)
}

/// Updates the session status in the database.
pub async fn update_session_status(
    pool: &DbPool,
    chat_id: Uuid,
    session_id: Uuid,
    status: crate::models::agent_session::SessionStatus,
    error_message: Option<String>,
) -> Result<()> {
    tracing::info!(
        chat_id = %chat_id,
        session_id = %session_id,
        new_status = %status,
        error_message = ?error_message,
        "[ChatActor] update_session_status: Starting update"
    );

    let mut conn = pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

    let _ = queries::agent_sessions::update_session_status(&mut conn, session_id, status, error_message).await?;

    tracing::info!(
        chat_id = %chat_id,
        session_id = %session_id,
        new_status = %status,
        "[ChatActor] update_session_status: Successfully updated"
    );

    Ok(())
}

/// Starts a background task that sends periodic heartbeats to the database.
/// This keeps the session alive and indicates the agent is actively running.
pub fn start_heartbeat_task(
    chat_id: Uuid,
    pool: DbPool,
    session_id: Uuid,
) -> JoinHandle<()> {
    tracing::info!(
        chat_id = %chat_id,
        session_id = %session_id,
        "[ChatActor] Starting heartbeat task (30s interval)"
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            interval.tick().await;

            tracing::trace!(
                session_id = %session_id,
                "[ChatActor] Heartbeat: sending update"
            );

            let mut conn = match pool.acquire().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        session_id = %session_id,
                        error = %e,
                        "[ChatActor] Heartbeat: failed to acquire database connection"
                    );
                    continue;
                }
            };

            // Update heartbeat timestamp
            if let Err(e) = agent_sessions::update_heartbeat(&mut conn, session_id).await {
                tracing::warn!(
                    session_id = %session_id,
                    error = %e,
                    "[ChatActor] Heartbeat: failed to update heartbeat"
                );
            } else {
                tracing::trace!(
                    session_id = %session_id,
                    "[ChatActor] Heartbeat: successfully updated"
                );
            }
        }
    })
}
