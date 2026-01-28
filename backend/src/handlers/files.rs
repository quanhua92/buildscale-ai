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
        AddLinkHttp, AddTagHttp, CreateFileHttp, CreateFileRequest, CreateVersionHttp,
        CreateVersionRequest, FileNetworkSummary, FileWithContent, SearchResult,
        SemanticSearchHttp, UpdateFileHttp,
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
        name = %request.name,
        "Creating new file",
    );

    let mut conn = acquire_db_connection(&state, "create_file").await?;

    let result = file_services::create_file_with_content(
        &mut conn,
        &state.storage,
        CreateFileRequest {
            workspace_id: workspace_access.workspace_id,
            parent_id: request.parent_id,
            author_id: auth_user.id,
            name: request.name,
            slug: request.slug,
            path: request.path,
            is_virtual: request.is_virtual,
            is_remote: request.is_remote,
            permission: request.permission,
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

    let result = file_services::get_file_with_content(&mut conn, &state.storage, file_id)
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
    use crate::models::requests::UpdateFileRequest;

    let mut conn = acquire_db_connection(&state, "update_file").await?;

    let update_request = UpdateFileRequest {
        parent_id: request.parent_id,
        name: request.name,
        slug: request.slug,
        is_virtual: request.is_virtual,
        is_remote: request.is_remote,
        permission: request.permission,
    };

    let result = file_services::update_file(&mut conn, file_id, update_request)
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

    file_services::soft_delete_file(&mut conn, &state.storage, file_id)
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

    let result = file_services::restore_file(&mut conn, &state.storage, file_id)
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
// TAGGING HANDLERS
// ============================================================================

/// POST /api/v1/workspaces/:id/files/:file_id/tags
///
/// Adds a tag to a file.
pub async fn add_tag(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<AddTagHttp>,
) -> Result<Json<serde_json::Value>> {
    let mut conn = acquire_db_connection(&state, "add_tag").await?;

    file_services::add_tag(&mut conn, file_id, &request.tag)
        .await
        .inspect_err(|e| log_handler_error("add_tag", e))?;

    Ok(Json(serde_json::json!({ "message": "Tag added successfully" })))
}

/// DELETE /api/v1/workspaces/:id/files/:file_id/tags/:tag
///
/// Removes a tag from a file.
pub async fn remove_tag(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id, tag)): Path<(Uuid, Uuid, String)>,
) -> Result<Json<serde_json::Value>> {
    let mut conn = acquire_db_connection(&state, "remove_tag").await?;

    file_services::remove_tag(&mut conn, file_id, &tag)
        .await
        .inspect_err(|e| log_handler_error("remove_tag", e))?;

    Ok(Json(serde_json::json!({ "message": "Tag removed successfully" })))
}

/// GET /api/v1/workspaces/:id/files/tags/:tag
///
/// Lists files by tag in a workspace.
pub async fn list_files_by_tag(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, tag)): Path<(Uuid, String)>,
) -> Result<Json<Vec<crate::models::files::File>>> {
    let mut conn = acquire_db_connection(&state, "list_files_by_tag").await?;

    let result = file_services::list_files_by_tag(&mut conn, workspace_access.workspace_id, &tag)
        .await
        .inspect_err(|e| log_handler_error("list_files_by_tag", e))?;

    Ok(Json(result))
}

// ============================================================================
// LINKING HANDLERS
// ============================================================================

/// POST /api/v1/workspaces/:id/files/:file_id/links
///
/// Creates a link between two files.
pub async fn create_link(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<AddLinkHttp>,
) -> Result<Json<serde_json::Value>> {
    let mut conn = acquire_db_connection(&state, "create_link").await?;

    file_services::link_files(&mut conn, file_id, request.target_file_id)
        .await
        .inspect_err(|e| log_handler_error("create_link", e))?;

    Ok(Json(serde_json::json!({ "message": "Link created successfully" })))
}

/// DELETE /api/v1/workspaces/:id/files/:file_id/links/:target_id
///
/// Removes a link between two files.
pub async fn remove_link(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id, target_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    let mut conn = acquire_db_connection(&state, "remove_link").await?;

    file_services::remove_link(&mut conn, file_id, target_id)
        .await
        .inspect_err(|e| log_handler_error("remove_link", e))?;

    Ok(Json(serde_json::json!({ "message": "Link removed successfully" })))
}

/// GET /api/v1/workspaces/:id/files/:file_id/network
///
/// Gets the local network summary for a file (tags, outbound links, backlinks).
pub async fn get_file_network(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<FileNetworkSummary>> {
    let mut conn = acquire_db_connection(&state, "get_file_network").await?;

    let result = file_services::get_file_network(&mut conn, file_id)
        .await
        .inspect_err(|e| log_handler_error("get_file_network", e))?;

    Ok(Json(result))
}

// ============================================================================
// SEARCH HANDLER
// ============================================================================

/// POST /api/v1/workspaces/:id/search
///
/// Performs semantic search across all files in the workspace.
pub async fn semantic_search(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Json(request): Json<SemanticSearchHttp>,
) -> Result<Json<Vec<SearchResult>>> {
    let mut conn = acquire_db_connection(&state, "semantic_search").await?;

    let results = file_services::semantic_search(&mut conn, workspace_access.workspace_id, request)
        .await
        .inspect_err(|e| log_handler_error("semantic_search", e))?;

    Ok(Json(results))
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
) -> Result<Json<crate::models::requests::FileWithContent>> {
    let mut conn = acquire_db_connection(&state, "create_version").await?;

    let version = file_services::create_version(
        &mut conn,
        &state.storage,
        file_id,
        CreateVersionRequest {
            author_id: Some(auth_user.id),
            branch: request.branch,
            content: request.content.clone(),
            app_data: request.app_data,
        },
    )
    .await
    .inspect_err(|e| log_handler_error("create_version", e))?;

    // Fetch the file with content to return in response
    let file = crate::queries::files::get_file_by_id(&mut conn, file_id).await?;

    Ok(Json(crate::models::requests::FileWithContent {
        file,
        latest_version: version,
        content: request.content,
    }))
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
        Error::Sqlx(e)
    })
}
