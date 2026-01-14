//! Workspace access control middleware
//!
//! This module provides middleware for validating workspace membership
//! and ownership for protected workspace routes.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;
use crate::{
    middleware::auth::AuthenticatedUser,
    state::AppState,
    error::{Error, Result},
    services::workspaces,
};

/// Workspace access context added to request extensions
///
/// This struct is added to request extensions by the workspace access middleware
/// after successful validation of workspace membership/ownership.
#[derive(Debug, Clone)]
pub struct WorkspaceAccess {
    /// The workspace ID being accessed
    pub workspace_id: Uuid,
    /// The authenticated user's ID
    pub user_id: Uuid,
    /// Whether the user is the workspace owner
    pub is_owner: bool,
    /// Whether the user is a workspace member (owners are always members)
    pub is_member: bool,
}

/// Middleware to validate workspace access control
///
/// This middleware:
/// 1. Extracts workspace_id from request path parameters
/// 2. Validates user is owner or member of the workspace
/// 3. Adds WorkspaceAccess to request extensions for handler use
/// 4. Returns 403 if user cannot access the workspace
///
/// # Token Sources
/// - Requires JWT authentication to be already validated (runs after jwt_auth_middleware)
///
/// # Behavior
/// 1. Extracts workspace_id from URL path (e.g., /api/v1/workspaces/{workspace_id})
/// 2. Validates authenticated user has access (owner OR member)
/// 3. Adds WorkspaceAccess with ownership/membership flags to extensions
/// 4. Returns 403 Forbidden if user lacks access
///
/// # Usage
/// Apply this middleware to protected workspace routes using `route_layer()`:
///
/// ```ignore
/// Router::new()
///     .route("/workspaces/:id", get(get_workspace))
///     .route_layer(middleware::from_fn_with_state(
///         state.clone(),
///         workspace_access_middleware,
///     ))
/// ```
pub async fn workspace_access_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    // Extract workspace_id from path
    let workspace_id = extract_workspace_id(&request)?;

    // Extract authenticated user from JWT middleware
    let auth_user = request.extensions()
        .get::<AuthenticatedUser>()
        .ok_or_else(|| Error::Authentication(
            "User not authenticated".to_string()
        ))?;

    // Acquire database connection
    let mut conn = state.pool.acquire().await
        .map_err(|e| Error::Internal(format!("Failed to acquire database connection: {}", e)))?;

    // Check workspace access using service method (authorization logic in service layer)
    let (is_owner, is_member) = workspaces::check_workspace_access(
        &mut conn,
        workspace_id,
        auth_user.id,
    ).await?;

    // Add workspace access context to extensions
    let access = WorkspaceAccess {
        workspace_id,
        user_id: auth_user.id,
        is_owner,
        is_member,
    };
    request.extensions_mut().insert(access);

    // Continue to handler
    Ok(next.run(request).await)
}

/// Extract workspace_id from request path
///
/// Supports paths like:
/// - /api/v1/workspaces/{workspace_id}
/// - /api/v1/workspaces/{workspace_id}/members
///
/// # Returns
/// * `Ok(Uuid)` - The workspace ID if found and valid
/// * `Err(Error)` - Validation error if workspace_id not found or invalid
fn extract_workspace_id<B>(request: &Request<B>) -> Result<Uuid> {
    let path = request.uri().path();

    // Split path and find the workspace ID segment
    let segments: Vec<&str> = path.split('/').collect();

    // Handle two cases:
    // 1. /api/v1/workspaces/{workspace_id} - full path (not nested)
    // 2. /{workspace_id} - nested router path (prefix already stripped)
    let workspace_id_str = if let Some(pos) = segments.iter().position(|&s| s == "workspaces") {
        // Case 1: Full path, get segment after "workspaces"
        segments.get(pos + 1)
    } else {
        // Case 2: Nested router, first non-empty segment is the workspace ID
        segments.iter().find(|s| !s.is_empty())
    }
    .ok_or_else(|| Error::Validation(
        crate::error::ValidationErrors::Single {
            field: "workspace_id".to_string(),
            message: "Workspace ID not found in path".to_string(),
        }
    ))?;

    Uuid::parse_str(workspace_id_str).map_err(|_| Error::Validation(
        crate::error::ValidationErrors::Single {
            field: "workspace_id".to_string(),
            message: "Invalid workspace ID format".to_string(),
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_workspace_id_from_path() {
        // Test basic path
        let request = Request::builder()
            .uri("/api/v1/workspaces/123e4567-e89b-12d3-a456-426614174000")
            .body("test body")
            .unwrap();

        let result = extract_workspace_id(&request);
        assert!(result.is_ok());
        let workspace_id = result.unwrap();
        assert_eq!(workspace_id.to_string(), "123e4567-e89b-12d3-a456-426614174000");
    }

    #[test]
    fn test_extract_workspace_id_with_trailing_path() {
        // Test path with additional segments
        let request = Request::builder()
            .uri("/api/v1/workspaces/123e4567-e89b-12d3-a456-426614174000/members")
            .body("test body")
            .unwrap();

        let result = extract_workspace_id(&request);
        assert!(result.is_ok());
        let workspace_id = result.unwrap();
        assert_eq!(workspace_id.to_string(), "123e4567-e89b-12d3-a456-426614174000");
    }

    #[test]
    fn test_extract_workspace_id_missing() {
        // Test path without workspaces segment
        let request = Request::builder()
            .uri("/api/v1/users/123")
            .body("test body")
            .unwrap();

        let result = extract_workspace_id(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_workspace_id_invalid_uuid() {
        // Test path with invalid UUID
        let request = Request::builder()
            .uri("/api/v1/workspaces/not-a-uuid")
            .body("test body")
            .unwrap();

        let result = extract_workspace_id(&request);
        assert!(result.is_err());
    }
}
