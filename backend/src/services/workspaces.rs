use crate::DbConn;
use crate::{
    error::{Error, Result, ValidationErrors},
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
    validation::{validate_workspace_name, validate_required_string},
};
use sqlx::Acquire;
use uuid::Uuid;

/// Creates a workspace with default roles and owner as admin
///
/// This operation ensures atomicity - workspace creation, role creation, and
/// owner membership assignment all succeed together or fail together.
/// No orphaned workspaces or incomplete setups are left behind.
pub async fn create_workspace(conn: &mut DbConn, request: CreateWorkspaceRequest) -> Result<CompleteWorkspaceResult> {
    // Validate workspace name format and constraints
    validate_workspace_name(&request.name)?;

    // Start a transaction for atomic workspace creation
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // Create the workspace within transaction with validated and sanitized name
    validate_workspace_name(&request.name)?;
    let new_workspace = NewWorkspace {
        name: validate_required_string(&request.name, "Workspace name")?,
        owner_id: request.owner_id,
    };
    let workspace = workspaces::create_workspace(&mut tx, new_workspace).await?;

    // Create default roles for the workspace within transaction
    let created_roles = roles::create_default_roles(&mut tx, workspace.id).await?;

    // Find the admin role
    let admin_role = created_roles
        .iter()
        .find(|role| role.name == ADMIN_ROLE)
        .ok_or_else(|| Error::Internal("Admin role not created properly".to_string()))?;

    // Add owner as admin member within transaction
    let owner_membership_data = NewWorkspaceMember {
        workspace_id: workspace.id,
        user_id: request.owner_id,
        role_id: admin_role.id,
    };
    let owner_membership = workspace_members::create_workspace_member(&mut tx, owner_membership_data).await?;

    // Commit the transaction - workspace, roles, and membership are now persisted atomically
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(CompleteWorkspaceResult {
        workspace,
        roles: created_roles,
        owner_membership: owner_membership.clone(),
        members: vec![owner_membership],
    })
}

/// Creates a workspace with default roles and multiple initial members
///
/// This operation ensures atomicity - workspace creation, role creation, owner
/// membership, and all additional member assignments succeed together or
/// fail together. No incomplete workspaces or member setups are left behind.
pub async fn create_workspace_with_members(conn: &mut DbConn, request: CreateWorkspaceWithMembersRequest) -> Result<CompleteWorkspaceResult> {
    // Validate workspace name format and constraints
    validate_workspace_name(&request.name)?;

    // Start a transaction for atomic workspace creation with members
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // Create the workspace within transaction with validated and sanitized name
    let new_workspace = NewWorkspace {
        name: validate_required_string(&request.name, "Workspace name")?,
        owner_id: request.owner_id,
    };
    let workspace = workspaces::create_workspace(&mut tx, new_workspace).await?;

    // Create default roles for the workspace within transaction
    let created_roles = roles::create_default_roles(&mut tx, workspace.id).await?;

    // Find the admin role
    let admin_role = created_roles
        .iter()
        .find(|role| role.name == ADMIN_ROLE)
        .ok_or_else(|| Error::Internal("Admin role not created properly".to_string()))?;

    // Add owner as admin member within transaction
    let owner_membership_data = NewWorkspaceMember {
        workspace_id: workspace.id,
        user_id: request.owner_id,
        role_id: admin_role.id,
    };
    let owner_membership = workspace_members::create_workspace_member(&mut tx, owner_membership_data).await?;
    let mut all_members = vec![owner_membership.clone()];

    // Add additional members within transaction
    for member_request in request.members {
        // Skip if trying to add owner again
        if member_request.user_id == request.owner_id {
            continue;
        }

        // Validate role name is not empty
        if member_request.role_name.trim().is_empty() {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "role_name".to_string(),
                message: "Role name cannot be empty".to_string(),
            }));
        }

        // Find the role by name within transaction
        let role = roles::get_role_by_name(&mut tx, workspace.id, &member_request.role_name).await?;

        // Create member within transaction
        let member_data = NewWorkspaceMember {
            workspace_id: workspace.id,
            user_id: member_request.user_id,
            role_id: role.id,
        };
        let member = workspace_members::create_workspace_member(&mut tx, member_data).await?;
        all_members.push(member);
    }

    // Commit the transaction - workspace, roles, and all memberships are now persisted atomically
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(CompleteWorkspaceResult {
        workspace,
        roles: created_roles,
        owner_membership: owner_membership.clone(),
        members: all_members,
    })
}

/// Updates workspace ownership (transfers to new owner)
///
/// This operation ensures atomicity - member role assignment/updating and
/// workspace ownership transfer succeed together or fail together.
/// No partial ownership transfers are left behind.
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
        return Err(Error::Validation(ValidationErrors::Single {
            field: "new_owner_id".to_string(),
            message: "Cannot transfer ownership to yourself".to_string(),
        }));
    }

    // Start a transaction for atomic ownership transfer
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // Get admin role to ensure new owner has admin access within transaction
    let admin_role = roles::get_role_by_name(&mut tx, workspace_id, ADMIN_ROLE).await?;

    // Add new owner as admin member if not already a member within transaction
    let existing_member = workspace_members::get_workspace_member_optional(
        &mut tx,
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
        workspace_members::create_workspace_member(&mut tx, new_member_data).await?;
    } else {
        // Update existing member's role to admin within transaction
        workspace_members::update_workspace_member(
            &mut tx,
            workspace_id,
            new_owner_id,
            crate::models::workspace_members::UpdateWorkspaceMember {
                role_id: Some(admin_role.id),
            },
        )
        .await?;
    }

    // Update the workspace owner within transaction
    let update_workspace = crate::models::workspaces::UpdateWorkspace {
        name: None,
        owner_id: Some(new_owner_id),
    };

    let updated_workspace = workspaces::update_workspace(&mut tx, workspace_id, update_workspace).await?;

    // Commit the transaction - ownership transfer and member role are now persisted atomically
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(updated_workspace)
}

/// Updates workspace details (name)
///
/// Only the workspace owner can update workspace details.
pub async fn update_workspace(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    update: crate::models::requests::UpdateWorkspaceRequest,
) -> Result<Workspace> {
    // Validate ownership
    if !workspaces::is_workspace_owner(conn, workspace_id, user_id).await? {
        return Err(Error::Forbidden(
            "You are not the owner of this workspace".to_string(),
        ));
    }

    // Validate workspace name
    validate_workspace_name(&update.name)?;

    // Build update struct
    let workspace_update = crate::models::workspaces::UpdateWorkspace {
        name: Some(validate_required_string(&update.name, "Workspace name")?),
        owner_id: None,  // Ownership changes use update_workspace_owner
    };

    let updated = workspaces::update_workspace(conn, workspace_id, workspace_update).await?;
    Ok(updated)
}

// Essential read methods (kept from original)
/// Gets a workspace by ID
pub async fn get_workspace(conn: &mut DbConn, id: Uuid) -> Result<Workspace> {
    let workspace = workspaces::get_workspace_by_id(conn, id).await?;
    Ok(workspace)
}

/// Lists all workspaces for a specific user (owner or member)
pub async fn list_user_workspaces(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<Workspace>> {
    // Get workspaces where user is owner
    let owned_workspaces = workspaces::get_workspaces_by_owner(conn, user_id).await?;

    // Get workspaces where user is a member
    let member_workspace_ids = workspace_members::get_workspace_ids_by_user(conn, user_id).await?;

    let mut all_workspaces = owned_workspaces;

    // Fetch member workspaces and deduplicate
    for workspace_id in member_workspace_ids {
        if let Ok(workspace) = workspaces::get_workspace_by_id(conn, workspace_id).await {
            if !all_workspaces.iter().any(|w| w.id == workspace.id) {
                all_workspaces.push(workspace);
            }
        }
    }

    Ok(all_workspaces)
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