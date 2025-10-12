use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::workspace_members::{WorkspaceMember},
    queries::{workspace_members, workspaces, roles},
};
use uuid::Uuid;

// Essential read methods that are still needed
/// Lists all members in a workspace
pub async fn list_workspace_members(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<WorkspaceMember>> {
    // Validate that the workspace exists
    let _workspace = workspaces::get_workspace_by_id(conn, workspace_id).await?;

    let members = workspace_members::list_workspace_members(conn, workspace_id).await?;
    Ok(members)
}

/// Lists all workspaces that a user is a member of
pub async fn list_user_workspaces(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<WorkspaceMember>> {
    let memberships = workspace_members::list_user_workspaces(conn, user_id).await?;
    Ok(memberships)
}

/// Gets a workspace member by workspace ID and user ID
pub async fn get_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<WorkspaceMember> {
    let member = workspace_members::get_workspace_member(conn, workspace_id, user_id).await?;
    Ok(member)
}

/// Gets a workspace member optionally (may not exist)
pub async fn get_workspace_member_optional(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<Option<WorkspaceMember>> {
    let member = workspace_members::get_workspace_member_optional(conn, workspace_id, user_id).await?;
    Ok(member)
}

/// Checks if a user is a member of a workspace
pub async fn is_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool> {
    let is_member = workspace_members::is_workspace_member(conn, workspace_id, user_id).await?;
    Ok(is_member)
}

// Limited update methods for specific cases
/// Updates a workspace member's role with validation
pub async fn update_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    update_member: crate::models::workspace_members::UpdateWorkspaceMember,
) -> Result<WorkspaceMember> {
    // Validate that the workspace exists
    let _workspace = workspaces::get_workspace_by_id(conn, workspace_id).await?;

    // Validate that the new role exists and belongs to the workspace
    if let Some(role_id) = update_member.role_id {
        let role = roles::get_role_by_id(conn, role_id).await?;
        if role.workspace_id != workspace_id {
            return Err(Error::Validation(
                "Role does not belong to the specified workspace".to_string(),
            ));
        }
    }

    // Check if the member exists
    let _existing_member = workspace_members::get_workspace_member(conn, workspace_id, user_id).await?;

    // Update the member
    let updated_member = workspace_members::update_workspace_member(
        conn,
        workspace_id,
        user_id,
        update_member,
    )
    .await?;

    Ok(updated_member)
}

/// Removes a member from a workspace
pub async fn remove_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<u64> {
    // Validate that the workspace exists
    let workspace = workspaces::get_workspace_by_id(conn, workspace_id).await?;

    // Prevent the owner from being removed as a member
    if workspace.owner_id == user_id {
        return Err(Error::Validation(
            "Cannot remove the workspace owner as a member".to_string(),
        ));
    }

    // Check if the member exists
    let _existing_member = workspace_members::get_workspace_member(conn, workspace_id, user_id).await?;

    // Remove the member
    let rows_affected = workspace_members::delete_workspace_member(conn, workspace_id, user_id).await?;

    if rows_affected == 0 {
        return Err(Error::NotFound("Workspace member not found".to_string()));
    }

    Ok(rows_affected)
}

// Low-level method used by comprehensive workspace creation
/// Creates a workspace member (internal use by comprehensive methods)
pub async fn create_workspace_member(
    conn: &mut DbConn,
    new_member: crate::models::workspace_members::NewWorkspaceMember,
) -> Result<WorkspaceMember> {
    // Validate that the workspace exists
    let _workspace = workspaces::get_workspace_by_id(conn, new_member.workspace_id).await?;

    // Validate that the role exists and belongs to the workspace
    let role = roles::get_role_by_id(conn, new_member.role_id).await?;
    if role.workspace_id != new_member.workspace_id {
        return Err(Error::Validation(
            "Role does not belong to the specified workspace".to_string(),
        ));
    }

    // Check if user is already a member of the workspace
    let existing_member = workspace_members::get_workspace_member_optional(
        conn,
        new_member.workspace_id,
        new_member.user_id,
    )
    .await?;

    if existing_member.is_some() {
        return Err(Error::Validation(
            "User is already a member of this workspace".to_string(),
        ));
    }

    // Create the workspace member
    let member = workspace_members::create_workspace_member(conn, new_member).await?;

    Ok(member)
}

/// Validates that a user can perform an action in a workspace based on their role
pub async fn validate_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    required_permission: &str,
) -> Result<bool> {
    // Check if user is the owner (owners have all permissions)
    if workspaces::is_workspace_owner(conn, workspace_id, user_id).await? {
        return Ok(true);
    }

    // Get the user's membership
    let member = workspace_members::get_workspace_member_optional(conn, workspace_id, user_id).await?;

    if let Some(membership) = member {
        // Get the role details
        let role = roles::get_role_by_id(conn, membership.role_id).await?;

        // Here you would implement your permission checking logic
        // For now, we'll do a simple role name check
        // In a real implementation, you'd have a permissions system
        match role.name.to_lowercase().as_str() {
            "admin" | "owner" => Ok(true),
            "editor" if required_permission == "read" => Ok(true),
            "editor" if required_permission == "write" => Ok(true),
            "viewer" if required_permission == "read" => Ok(true),
            _ => Ok(false),
        }
    } else {
        Ok(false) // User is not a member
    }
}