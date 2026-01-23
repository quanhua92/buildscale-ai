//! Workspace Member Management handlers
//!
//! This module provides HTTP handlers for workspace member operations.
//! Handlers follow the thin-layer pattern: they validate inputs, delegate to services,
//! and return responses. All business logic is in the service layer.

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use uuid::Uuid;
use crate::{
    error::{Error, Result},
    middleware::auth::AuthenticatedUser,
    middleware::workspace_access::WorkspaceAccess,
    models::workspace_members::{AddMemberRequest, UpdateMemberRoleRequest},
    services::workspace_members,
    state::AppState,
};

// ============================================================================
// LIST MEMBERS
// ============================================================================

/// GET /api/v1/workspaces/:id/members
///
/// Lists all members in a workspace with detailed user and role information.
/// Requires workspace membership with members:read permission.
///
/// # Parameters
/// - `id`: Workspace UUID
///
/// # Headers
/// - Authorization: Bearer <access_token>
///
/// # Returns
/// JSON response containing detailed members list.
///
/// # HTTP Status Codes
/// - `200 OK`: Members retrieved successfully
/// - `403 FORBIDDEN`: Insufficient permissions
/// - `404 NOT_FOUND`: Workspace not found
pub async fn list_members(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        operation = "list_members",
        workspace_id = %workspace_id,
        requester_id = %auth_user.id,
        "Listing workspace members",
    );

    let mut conn = acquire_db_connection(&state, "list_members").await?;

    let members = workspace_members::list_members(&mut conn, workspace_id, auth_user.id)
        .await
        .inspect_err(|e| log_handler_error("list_members", e))?;

    tracing::info!(
        operation = "list_members",
        workspace_id = %workspace_id,
        count = members.len(),
        "Members listed successfully",
    );

    Ok(Json(serde_json::json!({
        "members": members,
        "count": members.len(),
    })))
}

// ============================================================================
// GET MY MEMBERSHIP
// ============================================================================

/// GET /api/v1/workspaces/:id/members/me
///
/// Gets the current user's membership details in a workspace.
/// Requires workspace membership.
///
/// # Parameters
/// - `id`: Workspace UUID
///
/// # Headers
/// - Authorization: Bearer <access_token>
///
/// # Returns
/// JSON response containing detailed membership information.
///
/// # HTTP Status Codes
/// - `200 OK`: Membership retrieved successfully
/// - `404 NOT_FOUND`: Workspace not found or user is not a member
pub async fn get_my_membership(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        operation = "get_my_membership",
        workspace_id = %workspace_id,
        user_id = %auth_user.id,
        "Getting current user membership",
    );

    let mut conn = acquire_db_connection(&state, "get_my_membership").await?;

    let membership = workspace_members::get_my_membership(&mut conn, workspace_id, auth_user.id)
        .await
        .inspect_err(|e| log_handler_error("get_my_membership", e))?;

    tracing::info!(
        operation = "get_my_membership",
        workspace_id = %workspace_id,
        user_id = %auth_user.id,
        role = %membership.role_name,
        "Membership retrieved successfully",
    );

    Ok(Json(serde_json::json!({
        "member": membership,
    })))
}

// ============================================================================
// ADD MEMBER
// ============================================================================

/// POST /api/v1/workspaces/:id/members
///
/// Adds a new member to a workspace by email address with a specific role.
/// Requires members:write permission.
///
/// # Parameters
/// - `id`: Workspace UUID
///
/// # Headers
/// - Authorization: Bearer <access_token>
/// - Content-Type: application/json
///
/// # Request Body
/// - `email`: Email address of user to add
/// - `role_name`: Role name (e.g., "admin", "editor", "member", "viewer")
///
/// # Returns
/// JSON response containing the new member details.
///
/// # HTTP Status Codes
/// - `200 OK`: Member added successfully
/// - `400 BAD_REQUEST`: Invalid input (email format, role not found)
/// - `403 FORBIDDEN`: Insufficient permissions
/// - `404 NOT_FOUND`: Workspace or user not found
/// - `409 CONFLICT`: User is already a member
pub async fn add_member(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path(workspace_id): Path<Uuid>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<AddMemberRequest>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        operation = "add_member",
        workspace_id = %workspace_id,
        requester_id = %auth_user.id,
        target_email = %request.email,
        role_name = %request.role_name,
        "Adding member to workspace",
    );

    let mut conn = acquire_db_connection(&state, "add_member").await?;

    let member = workspace_members::add_member_by_email(&mut conn, workspace_id, auth_user.id, request)
        .await
        .inspect_err(|e| log_handler_error("add_member", e))?;

    tracing::info!(
        operation = "add_member",
        workspace_id = %workspace_id,
        new_member_id = %member.user_id,
        new_member_email = %member.email,
        role = %member.role_name,
        "Member added successfully",
    );

    Ok(Json(serde_json::json!({
        "member": member,
    })))
}

// ============================================================================
// UPDATE MEMBER ROLE
// ============================================================================

/// PATCH /api/v1/workspaces/:id/members/:user_id
///
/// Updates a workspace member's role.
/// Requires members:write permission.
/// Cannot modify the workspace owner's role.
///
/// # Parameters
/// - `id`: Workspace UUID
/// - `user_id`: UUID of user whose role is being updated
///
/// # Headers
/// - Authorization: Bearer <access_token>
/// - Content-Type: application/json
///
/// # Request Body
/// - `role_name`: New role name (e.g., "admin", "editor", "member", "viewer")
///
/// # Returns
/// JSON response containing the updated member details.
///
/// # HTTP Status Codes
/// - `200 OK`: Member role updated successfully
/// - `400 BAD_REQUEST`: Invalid role name
/// - `403 FORBIDDEN`: Insufficient permissions or attempting to modify owner
/// - `404 NOT_FOUND`: Workspace, member, or role not found
pub async fn update_member_role(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((workspace_id, target_user_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<UpdateMemberRoleRequest>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        operation = "update_member_role",
        workspace_id = %workspace_id,
        requester_id = %auth_user.id,
        target_user_id = %target_user_id,
        new_role = %request.role_name,
        "Updating member role",
    );

    let mut conn = acquire_db_connection(&state, "update_member_role").await?;

    let member = workspace_members::update_member_role(
        &mut conn,
        workspace_id,
        target_user_id,
        auth_user.id,
        request,
    )
    .await
    .inspect_err(|e| log_handler_error("update_member_role", e))?;

    tracing::info!(
        operation = "update_member_role",
        workspace_id = %workspace_id,
        user_id = %member.user_id,
        new_role = %member.role_name,
        "Member role updated successfully",
    );

    Ok(Json(serde_json::json!({
        "member": member,
    })))
}

// ============================================================================
// REMOVE MEMBER
// ============================================================================

/// DELETE /api/v1/workspaces/:id/members/:user_id
///
/// Removes a member from a workspace.
/// - Users can remove themselves (leave workspace).
/// - Requires members:write permission to remove other members.
/// - Cannot remove the workspace owner.
///
/// # Parameters
/// - `id`: Workspace UUID
/// - `user_id`: UUID of user to remove
///
/// # Headers
/// - Authorization: Bearer <access_token>
///
/// # Returns
/// JSON response confirming removal.
///
/// # HTTP Status Codes
/// - `200 OK`: Member removed successfully
/// - `403 FORBIDDEN`: Insufficient permissions or attempting to remove owner
/// - `404 NOT_FOUND`: Workspace or member not found
pub async fn remove_member(
    State(state): State<AppState>,
    Extension(_workspace_access): Extension<WorkspaceAccess>,
    Path((workspace_id, target_user_id)): Path<(Uuid, Uuid)>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        operation = "remove_member",
        workspace_id = %workspace_id,
        requester_id = %auth_user.id,
        target_user_id = %target_user_id,
        "Removing member from workspace",
    );

    let mut conn = acquire_db_connection(&state, "remove_member").await?;

    workspace_members::remove_member(&mut conn, workspace_id, target_user_id, auth_user.id)
        .await
        .inspect_err(|e| log_handler_error("remove_member", e))?;

    tracing::info!(
        operation = "remove_member",
        workspace_id = %workspace_id,
        user_id = %target_user_id,
        "Member removed successfully",
    );

    Ok(Json(serde_json::json!({
        "message": "Member removed successfully",
    })))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Helper to log handler errors with appropriate level
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

/// Helper to acquire database connection with consistent error logging
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
