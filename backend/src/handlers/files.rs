//! File management handlers
//!
//! This module provides HTTP handlers for file operations.
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
        CreateFileHttp, CreateFileRequest, CreateVersionHttp,
        FileWithContent, UpdateFileHttp,
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
    Extension(_auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateFileHttp>,
) -> Result<Json<FileWithContent>> {
    tracing::info!(
        operation = "create_file",
        workspace_id = %workspace_access.workspace_id,
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
            name: request.name,
            path: request.path,
            file_type: request.file_type,
            content: request.content,
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
/// Retrieves a file and its content.
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
    let mut conn = acquire_db_connection(&state, "update_file").await?;

    let result = file_services::update_file(
        &mut conn,
        &state.storage,
        file_id,
        request.name,
        request.parent_id,
    )
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
// PURGE FILE
// ============================================================================

/// DELETE /api/v1/workspaces/:id/files/:file_id/purge
///
/// Permanently deletes a file.
pub async fn purge_file(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    let mut conn = acquire_db_connection(&state, "purge_file").await?;

    let hashes = file_services::purge_file(&mut conn, workspace_access.workspace_id, file_id)
        .await
        .inspect_err(|e| log_handler_error("purge_file", e))?;

    // Notify worker for immediate physical cleanup
    if !hashes.is_empty() {
        if let Err(e) = state.archive_cleanup_tx.send(crate::state::ArchiveCleanupMessage {
            workspace_id: workspace_access.workspace_id,
            hashes,
        }) {
            tracing::warn!("Failed to send message to archive cleanup worker: {}", e);
        }
    }

    Ok(Json(serde_json::json!({ "message": "File permanently deleted" })))
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
// TAGGING HANDLERS (Obsidian-style - parse from content)
// ============================================================================

/// POST /api/v1/workspaces/:id/files/:file_id/tags
///
/// Adds a tag to a file (deprecated - tags are now parsed from content).
pub async fn add_tag(
    State(_state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, _file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "Tags are now parsed from content. Add #tag to your file content instead.",
        "deprecated": true
    })))
}

/// DELETE /api/v1/workspaces/:id/files/:file_id/tags/:tag
///
/// Removes a tag from a file (deprecated - tags are now parsed from content).
pub async fn remove_tag(
    State(_state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, _file_id, _tag)): Path<(Uuid, Uuid, String)>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "Tags are now parsed from content. Remove #tag from your file content instead.",
        "deprecated": true
    })))
}

/// GET /api/v1/workspaces/:id/files/tags/:tag
///
/// Lists files by tag (uses tags table for fast lookup).
pub async fn list_files_by_tag(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, tag)): Path<(Uuid, String)>,
) -> Result<Json<Vec<crate::models::files::File>>> {
    let mut conn = acquire_db_connection(&state, "list_files_by_tag").await?;

    let tag_lower = tag.to_lowercase();

    // Use the tags index for fast lookup
    let file_ids: Vec<Uuid> = sqlx::query_scalar!(
        "SELECT file_id FROM tags WHERE workspace_id = $1 AND tag = $2",
        workspace_access.workspace_id,
        tag_lower
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(Error::Sqlx)
    .inspect_err(|e| log_handler_error("list_files_by_tag", e))?;

    if file_ids.is_empty() {
        return Ok(Json(vec![]));
    }

    // Fetch files by IDs
    let files = file_services::get_files_by_ids(&mut conn, &file_ids)
        .await
        .inspect_err(|e| log_handler_error("list_files_by_tag", e))?;

    Ok(Json(files))
}

// ============================================================================
// LINKING HANDLERS (Obsidian-style - parse from content)
// ============================================================================

/// POST /api/v1/workspaces/:id/files/:file_id/links
///
/// Creates a link between two files (deprecated - links are now parsed from content).
pub async fn create_link(
    State(_state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, _file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "Links are now parsed from content. Add [[filename]] to your file content instead.",
        "deprecated": true
    })))
}

/// DELETE /api/v1/workspaces/:id/files/:file_id/links/:target_id
///
/// Removes a link between two files (deprecated - links are now parsed from content).
pub async fn remove_link(
    State(_state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, _file_id, _target_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "Links are now parsed from content. Remove [[filename]] from your file content instead.",
        "deprecated": true
    })))
}

/// GET /api/v1/workspaces/:id/files/:file_id/network
///
/// Gets the local network summary for a file (tags, outbound links, backlinks).
pub async fn get_file_network(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    let mut conn = acquire_db_connection(&state, "get_file_network").await?;

    // Get the file and its content
    let file_with_content = file_services::get_file_with_content(&mut conn, &state.storage, file_id)
        .await
        .inspect_err(|e| log_handler_error("get_file_network", e))?;

    let content_text = match file_with_content.content {
        serde_json::Value::String(ref s) => s.clone(),
        _ => file_services::extract_text_recursively(&file_with_content.content),
    };

    // Extract tags and links from content
    use crate::parsers::{extract_tags, extract_links};
    let tags = extract_tags(&content_text);
    let links = extract_links(&content_text);

    // Find backlinks (files that link to this file)
    let all_files = file_services::list_all_active_files(&mut conn, workspace_access.workspace_id)
        .await
        .inspect_err(|e| log_handler_error("get_file_network", e))?;

    let file_name = file_with_content.file.name.clone();
    let mut backlinks = Vec::new();

    for other_file in all_files {
        if other_file.id == file_id {
            continue;
        }
        if let Ok(other_content) = file_services::get_file_with_content(
            &mut conn,
            &state.storage,
            other_file.id,
        ).await {
            let other_text = match other_content.content {
                serde_json::Value::String(ref s) => s.clone(),
                _ => file_services::extract_text_recursively(&other_content.content),
            };
            let other_links = extract_links(&other_text);
            if other_links.iter().any(|l| l.to_lowercase() == file_name.to_lowercase()) {
                backlinks.push(other_file);
            }
        }
    }

    Ok(Json(serde_json::json!({
        "tags": tags,
        "outbound_links": links,
        "backlinks": backlinks.iter().map(|f| &f.name).collect::<Vec<_>>(),
    })))
}

// ============================================================================
// SEARCH HANDLER (deprecated - no semantic search)
// ============================================================================

/// POST /api/v1/workspaces/:id/search
///
/// Performs text search across all files in the workspace.
pub async fn semantic_search(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    // Extract query from request for text-based search
    let query = request.get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if query.is_empty() {
        return Ok(Json(serde_json::json!({
            "results": [],
            "message": "Provide a 'query' field for text search"
        })));
    }

    let mut conn = acquire_db_connection(&state, "semantic_search").await?;

    // Get all files and search in content
    let all_files = file_services::list_all_active_files(&mut conn, workspace_access.workspace_id)
        .await
        .inspect_err(|e| log_handler_error("semantic_search", e))?;

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for file in all_files {
        if let Ok(file_with_content) = file_services::get_file_with_content(
            &mut conn,
            &state.storage,
            file.id,
        ).await {
            let content_text = match file_with_content.content {
                serde_json::Value::String(ref s) => s.clone(),
                _ => file_services::extract_text_recursively(&file_with_content.content),
            };

            if content_text.to_lowercase().contains(&query_lower) {
                // Find context around match
                let idx = content_text.to_lowercase().find(&query_lower);
                let preview = if let Some(pos) = idx {
                    let start = pos.saturating_sub(50);
                    let end = (pos + query.len() + 50).min(content_text.len());
                    format!("...{}...", &content_text[start..end])
                } else {
                    String::new()
                };

                results.push(serde_json::json!({
                    "file": file,
                    "preview": preview,
                    "type": "text_match"
                }));
            }
        }
    }

    Ok(Json(serde_json::json!({
        "results": results,
        "query": query,
        "type": "text_search"
    })))
}

// ============================================================================
// CREATE VERSION (simplified - updates content)
// ============================================================================

/// POST /api/v1/workspaces/:id/files/:file_id/versions
///
/// Creates a new version for an existing file (updates content).
pub async fn create_version(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Extension(_auth_user): Extension<AuthenticatedUser>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CreateVersionHttp>,
) -> Result<Json<FileWithContent>> {
    let mut conn = acquire_db_connection(&state, "create_version").await?;

    // Update file content (this creates a new version in the simplified system)
    let updated_file = file_services::update_file_content(
        &mut conn,
        &state.storage,
        file_id,
        request.content.clone(),
    )
    .await
    .inspect_err(|e| log_handler_error("create_version", e))?;

    Ok(Json(FileWithContent {
        hash: updated_file.hash.clone().unwrap_or_default(),
        file: updated_file,
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
