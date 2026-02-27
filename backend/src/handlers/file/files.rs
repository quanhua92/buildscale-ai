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
    fs::models::FileType,
    models::requests::{
        CreateFileHttp, CreateFileRequest, CreateVersionHttp,
        FileWithContent, UpdateFileHttp,
    },
    fs::queries as file_queries,
    fs::services as file_services,
    state::{AppState, TagIndexMessage, LinkIndexMessage},
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
            name: request.name.clone(),
            path: request.path,
            file_type: request.file_type,
            content: request.content,
        },
    )
    .await
    .inspect_err(|e| log_handler_error("create_file", e))?;

    // Signal indexers for markdown documents
    let is_markdown = request.file_type == FileType::Document &&
        request.name.ends_with(".md");
    if is_markdown {
        let _ = state.tag_index_tx.send(TagIndexMessage {
            workspace_id: workspace_access.workspace_id,
            file_id: result.file.id,
        });
        let _ = state.link_index_tx.send(LinkIndexMessage {
            workspace_id: workspace_access.workspace_id,
            file_id: result.file.id,
        });
    }

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
) -> Result<Json<crate::fs::models::File>> {
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
) -> Result<Json<crate::fs::models::File>> {
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
) -> Result<Json<Vec<crate::fs::models::File>>> {
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
) -> Result<Json<Vec<crate::fs::models::File>>> {
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
    use crate::fs::parsers::{extract_tags, extract_links};
    let tags = extract_tags(&content_text);
    let links = extract_links(&content_text);

    // Find backlinks using links table (fast!)
    // Query files that have links pointing to this file's name
    // Note: Obsidian-style matching - [[file-b]] matches "file-b.md"
    let file_name = file_with_content.file.name.to_lowercase();
    // Strip .md extension for comparison
    let file_name_without_ext = file_name.strip_suffix(".md").unwrap_or(&file_name);

    let backlink_names: Vec<String> = sqlx::query_scalar!(
        r#"
        SELECT DISTINCT f.name
        FROM files f
        INNER JOIN links l ON l.source_file_id = f.id
        WHERE l.workspace_id = $1
          AND l.target_name = $2
          AND f.deleted_at IS NULL
        "#,
        workspace_access.workspace_id,
        file_name_without_ext
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(Error::Sqlx)
    .inspect_err(|e| log_handler_error("get_file_network", e))?;

    Ok(Json(serde_json::json!({
        "tags": tags,
        "outbound_links": links,
        "backlinks": backlink_names,
    })))
}

// ============================================================================
// SEARCH HANDLER (text-based search using ripgrep with fallback)
// ============================================================================

/// Check if ripgrep is available on the system
fn is_ripgrep_available() -> bool {
    std::process::Command::new("rg")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// POST /api/v1/workspaces/:id/search
///
/// Performs text search across all files in the workspace.
/// Uses ripgrep for fast search when available, falls back to database search otherwise.
pub async fn text_search(
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

    // Check if search directory exists
    let search_path = state.storage.get_workspace_path(workspace_access.workspace_id);
    if !search_path.exists() {
        return Ok(Json(serde_json::json!({
            "results": [],
            "query": query,
            "type": "text_search"
        })));
    }

    // Try ripgrep first (fast), fall back to database search if unavailable
    if is_ripgrep_available() {
        text_search_with_ripgrep(&state, workspace_access.workspace_id, &search_path, query).await
    } else {
        tracing::info!("ripgrep not available, using database fallback for text_search");
        text_search_with_database(&state, workspace_access.workspace_id, query).await
    }
}

/// Fast text search using ripgrep
async fn text_search_with_ripgrep(
    state: &AppState,
    workspace_id: Uuid,
    search_path: &std::path::Path,
    query: &str,
) -> Result<Json<serde_json::Value>> {
    // Build ripgrep command for case-insensitive search
    let output = tokio::process::Command::new("rg")
        .arg("--json")                    // JSON output for easy parsing
        .arg("-i")                        // Case insensitive
        .arg("--max-count=1")             // One match per file is enough
        .arg("--no-heading")              // Don't group by file
        .arg("--")
        .arg(query)
        .arg(search_path)
        .output()
        .await;

    let stdout = match output {
        Ok(o) => {
            // Exit code 1 means no matches (not an error)
            if !o.status.success() && o.status.code() != Some(1) {
                tracing::warn!("ripgrep search failed: {}", String::from_utf8_lossy(&o.stderr));
                return Ok(Json(serde_json::json!({
                    "results": [],
                    "query": query,
                    "type": "text_search"
                })));
            }
            String::from_utf8_lossy(&o.stdout).to_string()
        }
        Err(e) => {
            tracing::warn!("Failed to run ripgrep: {}, falling back to database search", e);
            return text_search_with_database(state, workspace_id, query).await;
        }
    };

    // Parse ripgrep JSON output and collect file paths
    let mut file_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    for line in stdout.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json["type"] == "match" {
                if let Some(path) = json["data"]["path"]["text"].as_str() {
                    // Convert absolute path to relative path
                    let relative = path.strip_prefix(&search_path.to_string_lossy().to_string())
                        .unwrap_or(path)
                        .trim_start_matches('/');
                    file_paths.insert(relative.to_string());
                }
            }
        }
    }

    // Fetch file metadata from database
    let mut conn = acquire_db_connection(state, "text_search").await?;
    let mut results = Vec::new();

    for path in file_paths {
        // Get file by path
        if let Ok(Some(file)) = file_queries::get_file_by_path(
            &mut conn,
            workspace_id,
            &format!("/{}", path),
        ).await {
            // Get file content for preview
            if let Ok(file_with_content) = file_services::get_file_with_content(
                &mut conn,
                &state.storage,
                file.id,
            ).await {
                let content_text = match file_with_content.content {
                    serde_json::Value::String(ref s) => s.clone(),
                    _ => file_services::extract_text_recursively(&file_with_content.content),
                };

                // Find context around match
                let preview = build_preview(&content_text, query);

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

/// Fallback text search using database (slower but works without ripgrep)
async fn text_search_with_database(
    state: &AppState,
    workspace_id: Uuid,
    query: &str,
) -> Result<Json<serde_json::Value>> {
    let mut conn = acquire_db_connection(state, "text_search_database").await?;

    // Get all active files (limited to prevent excessive I/O)
    let all_files = file_services::list_all_active_files(&mut conn, workspace_id)
        .await
        .inspect_err(|e| log_handler_error("text_search_database", e))?;

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    // Limit search to first 100 files to prevent timeout
    for file in all_files.into_iter().take(100) {
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
                let preview = build_preview(&content_text, query);

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

/// Build a preview snippet around the first match
fn build_preview(content: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();
    let content_lower = content.to_lowercase();

    if let Some(pos) = content_lower.find(&query_lower) {
        let start = pos.saturating_sub(50);
        let end = (pos + query.len() + 50).min(content.len());
        format!("...{}...", &content[start..end])
    } else {
        String::new()
    }
}

// ============================================================================
// CREATE VERSION (simplified - updates content)
// ============================================================================

/// POST /api/v1/workspaces/:id/files/:file_id/versions
///
/// Creates a new version for an existing file (updates content).
pub async fn create_version(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Extension(_auth_user): Extension<AuthenticatedUser>,
    Path((_workspace_id, file_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CreateVersionHttp>,
) -> Result<Json<FileWithContent>> {
    let mut conn = acquire_db_connection(&state, "create_version").await?;

    // Get file to check type
    let existing_file = file_queries::get_file_by_id(&mut conn, file_id)
        .await
        .inspect_err(|e| log_handler_error("create_version", e))?;

    // Update file content (this creates a new version in the simplified system)
    let updated_file = file_services::update_file_content(
        &mut conn,
        &state.storage,
        file_id,
        request.content.clone(),
    )
    .await
    .inspect_err(|e| log_handler_error("create_version", e))?;

    // Signal indexers for markdown documents
    let is_markdown = matches!(existing_file.file_type, FileType::Document) &&
        existing_file.name.ends_with(".md");
    if is_markdown {
        let _ = state.tag_index_tx.send(TagIndexMessage {
            workspace_id: workspace_access.workspace_id,
            file_id,
        });
        let _ = state.link_index_tx.send(LinkIndexMessage {
            workspace_id: workspace_access.workspace_id,
            file_id,
        });
    }

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
