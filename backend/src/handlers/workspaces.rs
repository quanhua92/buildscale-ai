//! Workspace CRUD handlers
//!
//! This module provides HTTP handlers for workspace operations.
//! Handlers follow the thin-layer pattern: they validate inputs, delegate to services,
//! and return responses. All business logic is in the service layer.

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use uuid::Uuid;
use crate::{
    error::{Error, Result},
    middleware::{auth::AuthenticatedUser, workspace_access::WorkspaceAccess},
    models::requests::{CreateWorkspaceRequest, UpdateWorkspaceRequest},
    services::workspaces,
    state::AppState,
};

// ============================================================================
// CREATE WORKSPACE
// ============================================================================

/// POST /api/v1/workspaces
///
/// Creates a new workspace with the authenticated user as owner.
/// No existing workspace membership required.
///
/// # Request Body
/// - `name`: Workspace name (1-100 characters)
///
/// # Returns
/// JSON response containing:
/// - `workspace`: The created workspace
/// - `roles`: Default roles created for the workspace
/// - `owner_membership`: Owner's workspace membership
///
/// # HTTP Status Codes
/// - `200 OK`: Workspace created successfully
/// - `400 BAD_REQUEST`: Validation error
/// - `500 INTERNAL_SERVER_ERROR`: Database error
pub async fn create_workspace(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateWorkspaceRequest>,
) -> Result<Json<serde_json::Value>> {
    let mut conn = state.pool.acquire().await
        .map_err(|e| Error::Internal(format!("Failed to acquire database connection: {}", e)))?;

    let result = workspaces::create_workspace(
        &mut conn,
        CreateWorkspaceRequest {
            name: request.name,
            owner_id: auth_user.id,
        },
    ).await?;

    Ok(Json(serde_json::json!({
        "workspace": result.workspace,
        "roles": result.roles,
        "owner_membership": result.owner_membership,
    })))
}

// ============================================================================
// LIST USER WORKSPACES
// ============================================================================

/// GET /api/v1/workspaces
///
/// Lists all workspaces where the authenticated user is owner or member.
///
/// # Returns
/// JSON response containing:
/// - `workspaces`: Array of workspaces
/// - `count`: Total number of workspaces
///
/// # HTTP Status Codes
/// - `200 OK`: Workspaces retrieved successfully
/// - `500 INTERNAL_SERVER_ERROR`: Database error
pub async fn list_workspaces(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<serde_json::Value>> {
    let mut conn = state.pool.acquire().await
        .map_err(|e| Error::Internal(format!("Failed to acquire database connection: {}", e)))?;

    let workspaces = workspaces::list_user_workspaces(&mut conn, auth_user.id).await?;

    Ok(Json(serde_json::json!({
        "workspaces": workspaces,
        "count": workspaces.len(),
    })))
}

// ============================================================================
// GET SINGLE WORKSPACE
// ============================================================================

/// GET /api/v1/workspaces/:id
///
/// Gets a single workspace by ID.
/// Requires workspace membership (owner or member).
///
/// # Parameters
/// - `id`: Workspace UUID
///
/// # Returns
/// JSON response containing the workspace.
///
/// # HTTP Status Codes
/// - `200 OK`: Workspace retrieved successfully
/// - `403 FORBIDDEN`: User is not a member of the workspace
/// - `404 NOT_FOUND`: Workspace not found
/// - `500 INTERNAL_SERVER_ERROR`: Database error
pub async fn get_workspace(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthenticatedUser>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,  // Already validated by middleware
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Middleware already validated access, now fetch the workspace
    let mut conn = state.pool.acquire().await
        .map_err(|e| Error::Internal(format!("Failed to acquire database connection: {}", e)))?;

    let workspace = workspaces::get_workspace(&mut conn, workspace_id).await?;

    Ok(Json(serde_json::json!({
        "workspace": workspace,
    })))
}

// ============================================================================
// UPDATE WORKSPACE
// ============================================================================

/// PATCH /api/v1/workspaces/:id
///
/// Updates workspace details (name).
/// Requires workspace ownership.
///
/// # Parameters
/// - `id`: Workspace UUID
///
/// # Request Body
/// - `name`: New workspace name (1-100 characters)
///
/// # Returns
/// JSON response containing the updated workspace.
///
/// # HTTP Status Codes
/// - `200 OK`: Workspace updated successfully
/// - `400 BAD_REQUEST`: Validation error
/// - `403 FORBIDDEN**: User is not the workspace owner
/// - `404 NOT_FOUND`: Workspace not found
/// - `500 INTERNAL_SERVER_ERROR`: Database error
pub async fn update_workspace(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<UpdateWorkspaceRequest>,
) -> Result<Json<serde_json::Value>> {
    // Middleware validated membership, now check ownership
    if !workspace_access.is_owner {
        return Err(Error::Forbidden(
            "Only the workspace owner can update workspace details".to_string(),
        ));
    }

    let mut conn = state.pool.acquire().await
        .map_err(|e| Error::Internal(format!("Failed to acquire database connection: {}", e)))?;

    let workspace = workspaces::update_workspace(
        &mut conn,
        workspace_id,
        auth_user.id,
        request,
    ).await?;

    Ok(Json(serde_json::json!({
        "workspace": workspace,
    })))
}

// ============================================================================
// DELETE WORKSPACE
// ============================================================================

/// DELETE /api/v1/workspaces/:id
///
/// Deletes a workspace.
/// Requires workspace ownership.
///
/// # Parameters
/// - `id`: Workspace UUID
///
/// # Returns
/// JSON response confirming deletion.
///
/// # HTTP Status Codes
/// - `200 OK`: Workspace deleted successfully
/// - `403 FORBIDDEN`: User is not the workspace owner
/// - `404 NOT_FOUND`: Workspace not found
/// - `500 INTERNAL_SERVER_ERROR`: Database error
pub async fn delete_workspace(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthenticatedUser>,
    Extension(workspace_access): Extension<WorkspaceAccess>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Middleware validated membership, now check ownership
    if !workspace_access.is_owner {
        return Err(Error::Forbidden(
            "Only the workspace owner can delete the workspace".to_string(),
        ));
    }

    let mut conn = state.pool.acquire().await
        .map_err(|e| Error::Internal(format!("Failed to acquire database connection: {}", e)))?;

    workspaces::delete_workspace(&mut conn, workspace_id).await?;

    Ok(Json(serde_json::json!({
        "message": "Workspace deleted successfully",
    })))
}
