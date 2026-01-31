//! Tool execution handlers
//!
//! This module provides HTTP handlers for the tool execution API.

use axum::{extract::{Extension, State}, Json};
use crate::{
    error::{Error, Result},
    middleware::workspace_access::WorkspaceAccess,
    models::requests::ToolRequest,
    tools,
    state::AppState,
};

/// POST /api/v1/workspaces/:id/tools
///
/// Executes a tool with given arguments.
///
/// # Authentication & Authorization
/// - Requires valid JWT token (via workspace_access_middleware)
/// - User must be a member of the workspace
///
/// # Request Body
/// ```json
/// {
///   "tool": "read",
///   "args": { "path": "/file.txt" }
/// }
/// ```
///
/// # Available Tools
/// - `ls`: List directory contents
///   - args: { "path": "/folder"?, "recursive": false? }
/// - `read`: Read file contents
///   - args: { "path": "/file.txt" }
/// - `write`: Write or update file
///   - args: { "path": "/file.txt", "content": {...} }
/// - `rm`: Delete file or folder
///   - args: { "path": "/file.txt" }
///
/// # Response
/// ```json
/// {
///   "success": true,
///   "result": { ... },
///   "error": null
/// }
/// ```
pub async fn execute_tool(
    State(state): State<AppState>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Json(request): Json<ToolRequest>,
) -> Result<Json<crate::models::requests::ToolResponse>> {
    tracing::info!(
        operation = "execute_tool",
        workspace_id = %workspace_access.workspace_id,
        user_id = %workspace_access.user_id,
        tool = %request.tool,
        "Executing tool",
    );

    let mut conn = acquire_db_connection(&state, "execute_tool").await?;

    let executor = tools::get_tool_executor(&request.tool)?;

    // TODO: Phase 3 - Extract ToolConfig from chat metadata
    // For now, use default config (plan_mode: true for safety)
    let config = tools::ToolConfig::default();

    let response = executor
        .execute(
            &mut conn,
            &state.storage,
            workspace_access.workspace_id,
            workspace_access.user_id,
            config,
            request.args,
        )
        .await
        .inspect_err(|e| log_handler_error("execute_tool", e))?;
    
    Ok(Json(response))
}

/// Helper to acquire database connection
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
        Error::Sqlx(e)
    })
}

/// Helper to log handler errors
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
