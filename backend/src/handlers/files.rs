//! File management handlers
//!
//! This module provides HTTP handlers for file and version operations.
//! Handlers follow the thin-layer pattern: they validate inputs, delegate to services,
//! and return responses.

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use uuid::Uuid;
use crate::{
    error::{Error, Result},
    middleware::auth::AuthenticatedUser,
    middleware::workspace_access::WorkspaceAccess,
    models::requests::{
        CreateFileHttp, CreateFileRequest, CreateVersionHttp, CreateVersionRequest, FileWithContent,
        UpdateFileHttp,
    },
    services::files as file_services,
    state::AppState,
};

// ============================================================================
// CREATE FILE
// ============================================================================

/// POST /api/v1/workspaces/:id/files
///
/// Creates a new file or folder in the workspace.
pub async fn create_file(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateFileHttp>,
) -> Result<Json<FileWithContent>> {
    tracing::info!(
        operation = "create_file",
        workspace_id = %workspace_access.workspace_id,
        user_id = %auth_user.id,
        slug = %request.slug,
        "Creating new file",
    );

    let mut conn = acquire_db_connection(&state, "create_file").await?;

    let result = file_services::create_file_with_content(
        &mut conn,
        CreateFileRequest {
            workspace_id: workspace_access.workspace_id,
            parent_id: request.parent_id,
            author_id: auth_user.id,
            slug: request.slug,
            file_type: request.file_type,
            content: request.content,
            app_data: request.app_data,
        },
    )
    .await
    .inspect_err(|e| log_handler_error("create_file", e))?;

    Ok(Json(result))
}

// ============================================================================
// GET FILE
// ============================================================================

/// GET /api/v1/workspaces/:id/files/:file_id
///
/// Retrieves a file and its latest version.
pub async fn get_file(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<FileWithContent>> {
    let mut conn = acquire_db_connection(&state, "get_file").await?;

    let result = file_services::get_file_with_content(&mut conn, file_id)
        .await
        .inspect_err(|e| log_handler_error("get_file", e))?;

    Ok(Json(result))
}

// ============================================================================
// UPDATE FILE
// ============================================================================

/// PATCH /api/v1/workspaces/:id/files/:file_id
///
/// Updates file metadata (move and/or rename).
pub async fn update_file(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateFileHttp>,
) -> Result<Json<crate::models::files::File>> {
    let mut conn = acquire_db_connection(&state, "update_file").await?;

    let result = file_services::move_or_rename_file(&mut conn, file_id, request)
        .await
        .inspect_err(|e| log_handler_error("update_file", e))?;

    Ok(Json(result))
}

// ============================================================================
// DELETE FILE
// ============================================================================

/// DELETE /api/v1/workspaces/:id/files/:file_id
///
/// Soft deletes a file. Folders must be empty.
pub async fn delete_file(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    let mut conn = acquire_db_connection(&state, "delete_file").await?;

    file_services::soft_delete_file(&mut conn, file_id)
        .await
        .inspect_err(|e| log_handler_error("delete_file", e))?;

    Ok(Json(serde_json::json!({ "message": "File deleted successfully" })))
}

// ============================================================================
// RESTORE FILE
// ============================================================================

/// POST /api/v1/workspaces/:id/files/:file_id/restore
///
/// Restores a soft-deleted file.
pub async fn restore_file(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<crate::models::files::File>> {
    let mut conn = acquire_db_connection(&state, "restore_file").await?;

    let result = file_services::restore_file(&mut conn, file_id)
        .await
        .inspect_err(|e| log_handler_error("restore_file", e))?;

    Ok(Json(result))
}

// ============================================================================
// LIST TRASH
// ============================================================================

/// GET /api/v1/workspaces/:id/files/trash
///
/// Lists all soft-deleted files in the workspace.
pub async fn list_trash(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
) -> Result<Json<Vec<crate::models::files::File>>> {
    let mut conn = acquire_db_connection(&state, "list_trash").await?;

    let result = file_services::list_trash(&mut conn, workspace_access.workspace_id)
        .await
        .inspect_err(|e| log_handler_error("list_trash", e))?;

    Ok(Json(result))
}

// ============================================================================
// CREATE VERSION
// ============================================================================

/// POST /api/v1/workspaces/:id/files/:file_id/versions
///
/// Creates a new version for an existing file.
pub async fn create_version(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CreateVersionHttp>,
) -> Result<Json<crate::models::files::FileVersion>> {
    let mut conn = acquire_db_connection(&state, "create_version").await?;

    let result = file_services::create_version(
        &mut conn,
        file_id,
        CreateVersionRequest {
            author_id: Some(auth_user.id),
            branch: request.branch,
            content: request.content,
            app_data: request.app_data,
        },
    )
    .await
    .inspect_err(|e| log_handler_error("create_version", e))?;

    Ok(Json(result))
}

// ============================================================================
// HELPERS
// ============================================================================

fn log_handler_error(operation: &str, e: &Error) {
    match e {
        Error::Validation(_) | Error::NotFound(_) | Error::Forbidden(_) | Error::Conflict(_) => {
            tracing::warn!(operation = operation, error = %e, "Handler operation failed");
        }
        _ => {
            tracing::error!(operation = operation, error = %e, "Handler operation failed");
        }
    }
}

async fn acquire_db_connection(state: &AppState, operation: &'static str) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>> {
    state.pool.acquire().await.map_err(|e| {
        tracing::error!(
            operation = operation,
            error_code = "DATABASE_ACQUISITION_FAILED",
            error = %e,
            "Failed to acquire database connection",
        );
        Error::Internal(format!("Failed to acquire database connection: {}", e))
    })
}
