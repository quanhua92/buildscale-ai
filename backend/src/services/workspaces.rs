use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::{
        requests::{
            CreateWorkspaceRequest, CreateWorkspaceWithMembersRequest,
            CompleteWorkspaceResult
        },
        workspaces::{NewWorkspace, Workspace},
        workspace_members::NewWorkspaceMember,
        roles::ADMIN_ROLE,
    },
    queries::{workspaces, workspace_members},
    services::roles,
};
use uuid::Uuid;

/// Creates a workspace with default roles and owner as admin
pub async fn create_workspace(conn: &mut DbConn, request: CreateWorkspaceRequest) -> Result<CompleteWorkspaceResult> {
    // Validate workspace name is not empty
    if request.name.trim().is_empty() {
        return Err(Error::Validation("Workspace name cannot be empty".to_string()));
    }

    // Validate workspace name length (maximum 100 characters)
    if request.name.len() > 100 {
        return Err(Error::Validation(
            "Workspace name must be less than 100 characters".to_string(),
        ));
    }

    // Create the workspace
    let new_workspace = NewWorkspace {
        name: request.name,
        owner_id: request.owner_id,
    };
    let workspace = workspaces::create_workspace(conn, new_workspace).await?;

    // Create default roles for the workspace
    let created_roles = roles::create_default_roles(conn, workspace.id).await?;

    // Find the admin role
    let admin_role = created_roles
        .iter()
        .find(|role| role.name == ADMIN_ROLE)
        .ok_or_else(|| Error::Internal("Admin role not created properly".to_string()))?;

    // Add owner as admin member
    let owner_membership_data = NewWorkspaceMember {
        workspace_id: workspace.id,
        user_id: request.owner_id,
        role_id: admin_role.id,
    };
    let owner_membership = workspace_members::create_workspace_member(conn, owner_membership_data).await?;

    Ok(CompleteWorkspaceResult {
        workspace,
        roles: created_roles,
        owner_membership: owner_membership.clone(),
        members: vec![owner_membership],
    })
}

/// Creates a workspace with default roles and multiple initial members
pub async fn create_workspace_with_members(conn: &mut DbConn, request: CreateWorkspaceWithMembersRequest) -> Result<CompleteWorkspaceResult> {
    // Validate workspace name is not empty
    if request.name.trim().is_empty() {
        return Err(Error::Validation("Workspace name cannot be empty".to_string()));
    }

    // Validate workspace name length (maximum 100 characters)
    if request.name.len() > 100 {
        return Err(Error::Validation(
            "Workspace name must be less than 100 characters".to_string(),
        ));
    }

    // Create the workspace
    let new_workspace = NewWorkspace {
        name: request.name,
        owner_id: request.owner_id,
    };
    let workspace = workspaces::create_workspace(conn, new_workspace).await?;

    // Create default roles for the workspace
    let created_roles = roles::create_default_roles(conn, workspace.id).await?;

    // Find the admin role
    let admin_role = created_roles
        .iter()
        .find(|role| role.name == ADMIN_ROLE)
        .ok_or_else(|| Error::Internal("Admin role not created properly".to_string()))?;

    // Add owner as admin member
    let owner_membership_data = NewWorkspaceMember {
        workspace_id: workspace.id,
        user_id: request.owner_id,
        role_id: admin_role.id,
    };
    let owner_membership = workspace_members::create_workspace_member(conn, owner_membership_data).await?;
    let mut all_members = vec![owner_membership.clone()];

    // Add additional members
    for member_request in request.members {
        // Skip if trying to add owner again
        if member_request.user_id == request.owner_id {
            continue;
        }

        // Find the role by name
        let role = roles::get_role_by_name(conn, workspace.id, &member_request.role_name).await?;

        // Create member
        let member_data = NewWorkspaceMember {
            workspace_id: workspace.id,
            user_id: member_request.user_id,
            role_id: role.id,
        };
        let member = workspace_members::create_workspace_member(conn, member_data).await?;
        all_members.push(member);
    }

    Ok(CompleteWorkspaceResult {
        workspace,
        roles: created_roles,
        owner_membership: owner_membership.clone(),
        members: all_members,
    })
}

/// Updates workspace ownership (transfers to new owner)
pub async fn update_workspace_owner(
    conn: &mut DbConn,
    workspace_id: Uuid,
    current_owner_id: Uuid,
    new_owner_id: Uuid,
) -> Result<Workspace> {
    // Validate current ownership
    if !workspaces::is_workspace_owner(conn, workspace_id, current_owner_id).await? {
        return Err(Error::Forbidden(
            "You are not the owner of this workspace".to_string(),
        ));
    }

    // Prevent transferring to the same user
    if current_owner_id == new_owner_id {
        return Err(Error::Validation(
            "Cannot transfer ownership to yourself".to_string(),
        ));
    }

    // Get admin role to ensure new owner has admin access
    let admin_role = roles::get_role_by_name(conn, workspace_id, ADMIN_ROLE).await?;

    // Add new owner as admin member if not already a member
    let existing_member = workspace_members::get_workspace_member_optional(
        conn,
        workspace_id,
        new_owner_id,
    )
    .await?;

    if existing_member.is_none() {
        let new_member_data = NewWorkspaceMember {
            workspace_id,
            user_id: new_owner_id,
            role_id: admin_role.id,
        };
        workspace_members::create_workspace_member(conn, new_member_data).await?;
    } else {
        // Update existing member's role to admin
        workspace_members::update_workspace_member(
            conn,
            workspace_id,
            new_owner_id,
            crate::models::workspace_members::UpdateWorkspaceMember {
                role_id: Some(admin_role.id),
            },
        )
        .await?;
    }

    // Update the workspace owner
    let update_workspace = crate::models::workspaces::UpdateWorkspace {
        name: None,
        owner_id: Some(new_owner_id),
    };

    let updated_workspace = workspaces::update_workspace(conn, workspace_id, update_workspace).await?;
    Ok(updated_workspace)
}

// Essential read methods (kept from original)
/// Gets a workspace by ID
pub async fn get_workspace(conn: &mut DbConn, id: Uuid) -> Result<Workspace> {
    let workspace = workspaces::get_workspace_by_id(conn, id).await?;
    Ok(workspace)
}

/// Lists all workspaces for a specific owner
pub async fn list_user_workspaces(conn: &mut DbConn, owner_id: Uuid) -> Result<Vec<Workspace>> {
    let workspaces = workspaces::get_workspaces_by_owner(conn, owner_id).await?;
    Ok(workspaces)
}

/// Lists all workspaces
pub async fn list_workspaces(conn: &mut DbConn) -> Result<Vec<Workspace>> {
    let workspaces = workspaces::list_workspaces(conn).await?;
    Ok(workspaces)
}

/// Validates that a user is the owner of a workspace
pub async fn validate_workspace_ownership(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool> {
    let is_owner = workspaces::is_workspace_owner(conn, workspace_id, user_id).await?;
    Ok(is_owner)
}

/// Checks if a user has permission to access a workspace (either as owner or member)
pub async fn can_access_workspace(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool> {
    // Check if user is the owner
    if workspaces::is_workspace_owner(conn, workspace_id, user_id).await? {
        return Ok(true);
    }

    // Check if user is a member
    let is_member = workspace_members::is_workspace_member(
        conn,
        workspace_id,
        user_id,
    )
    .await?;

    Ok(is_member)
}

/// Deletes a workspace by ID
pub async fn delete_workspace(conn: &mut DbConn, id: Uuid) -> Result<u64> {
    // Check if the workspace exists
    let workspace = workspaces::get_workspace_by_id_optional(conn, id).await?;

    if workspace.is_none() {
        return Err(Error::NotFound("Workspace not found".to_string()));
    }

    // Delete the workspace (cascade will handle related records)
    let rows_affected = workspaces::delete_workspace(conn, id).await?;

    if rows_affected == 0 {
        return Err(Error::NotFound("Workspace not found".to_string()));
    }

    Ok(rows_affected)
}