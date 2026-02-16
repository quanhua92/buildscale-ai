//! Chat files handlers
//!
//! This module provides HTTP handlers for chat file operations,
//! including listing recent chats and managing chat metadata.

use crate::error::{Error, Result};
use crate::models::files::FileType;
use crate::queries;
use crate::state::AppState;
use crate::middleware::auth::WorkspaceAccess;
use axum::extract::{Path, State};
use axum::extension::Extension;
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

/// List recent chats for a workspace.
///
/// Returns all chat files ordered by most recently updated, suitable for
/// displaying in a "Recent Chats" navigation sidebar.
///
/// # Response
///
/// Returns JSON array of chat files with metadata
#[tracing::instrument(skip_all)]
pub async fn list_chats(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>> {
    let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;

    let chat_files = queries::files::get_files_by_type(
        &mut conn,
        workspace_access.workspace_id,
        FileType::Chat,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to list chat files: {}", e);
        Error::Sqlx(e)
    })?;

    // Convert to JSON response with chat-specific metadata
    let chats: Vec<serde_json::Value> = chat_files
        .into_iter()
        .map(|file| {
            serde_json::json!({
                "id": file.id,
                "name": file.name,
                "path": file.path,
                "created_at": file.created_at,
                "updated_at": file.updated_at,
                "chat_id": file.id, // For convenience, the file_id is the chat_id
            })
        })
        .collect();

    tracing::debug!(
        workspace_id = %workspace_access.workspace_id,
        count = chats.len(),
        "Listed chat files for workspace"
    );

    Ok(Json(chats))
}
