use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::workspaces::{NewWorkspace, UpdateWorkspace, Workspace},
    queries::workspaces,
};
use uuid::Uuid;

/// Creates a new workspace with validation
pub async fn create_workspace(conn: &mut DbConn, new_workspace: NewWorkspace) -> Result<Workspace> {
    // Validate workspace name is not empty
    if new_workspace.name.trim().is_empty() {
        return Err(Error::Validation("Workspace name cannot be empty".to_string()));
    }

    // Validate workspace name length (maximum 100 characters)
    if new_workspace.name.len() > 100 {
        return Err(Error::Validation(
            "Workspace name must be less than 100 characters".to_string(),
        ));
    }

    // Create the workspace
    let workspace = workspaces::create_workspace(conn, new_workspace).await?;

    Ok(workspace)
}

/// Gets a workspace by ID
pub async fn get_workspace_by_id(conn: &mut DbConn, id: Uuid) -> Result<Workspace> {
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

/// Updates an existing workspace with validation
pub async fn update_workspace(
    conn: &mut DbConn,
    id: Uuid,
    update_workspace: UpdateWorkspace,
) -> Result<Workspace> {
    // Get the existing workspace first
    let existing_workspace = workspaces::get_workspace_by_id(conn, id).await?;

    // Validate new name if provided
    if let Some(ref name) = update_workspace.name {
        if name.trim().is_empty() {
            return Err(Error::Validation("Workspace name cannot be empty".to_string()));
        }

        if name.len() > 100 {
            return Err(Error::Validation(
                "Workspace name must be less than 100 characters".to_string(),
            ));
        }
    }

    // Validate new owner if provided
    if let Some(new_owner_id) = update_workspace.owner_id {
        // Prevent transferring ownership to the same user
        if new_owner_id == existing_workspace.owner_id {
            return Err(Error::Validation(
                "New owner must be different from current owner".to_string(),
            ));
        }

        // You might want to add additional validation here to ensure
        // the new owner exists and is a valid user
    }

    // Update the workspace
    let updated_workspace = workspaces::update_workspace(conn, id, update_workspace).await?;

    Ok(updated_workspace)
}

/// Deletes a workspace by ID
pub async fn delete_workspace(conn: &mut DbConn, id: Uuid) -> Result<u64> {
    // Check if the workspace exists
    let _workspace = workspaces::get_workspace_by_id(conn, id).await?;

    // Delete the workspace
    let rows_affected = workspaces::delete_workspace(conn, id).await?;

    if rows_affected == 0 {
        return Err(Error::NotFound("Workspace not found".to_string()));
    }

    Ok(rows_affected)
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
    let is_member = crate::queries::workspace_members::is_workspace_member(
        conn,
        workspace_id,
        user_id,
    )
    .await?;

    Ok(is_member)
}

/// Transfers workspace ownership to another user
pub async fn transfer_workspace_ownership(
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

    // Update the workspace owner
    let update_workspace = UpdateWorkspace {
        name: None,
        owner_id: Some(new_owner_id),
    };

    let updated_workspace = workspaces::update_workspace(conn, workspace_id, update_workspace).await?;

    Ok(updated_workspace)
}