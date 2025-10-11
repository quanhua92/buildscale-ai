use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::roles::{NewRole, Role, UpdateRole},
    queries::roles,
};
use uuid::Uuid;

/// Creates a new role with validation
pub async fn create_role(conn: &mut DbConn, new_role: NewRole) -> Result<Role> {
    // Validate role name is not empty
    if new_role.name.trim().is_empty() {
        return Err(Error::Validation("Role name cannot be empty".to_string()));
    }

    // Validate role name length (maximum 100 characters)
    if new_role.name.len() > 100 {
        return Err(Error::Validation(
            "Role name must be less than 100 characters".to_string(),
        ));
    }

    // Check if role with same name already exists in the workspace
    let existing_role = roles::get_role_by_workspace_and_name(
        conn,
        new_role.workspace_id,
        &new_role.name,
    )
    .await?;

    if existing_role.is_some() {
        return Err(Error::Conflict(format!(
            "Role '{}' already exists in this workspace",
            new_role.name
        )));
    }

    // Validate description length if provided (maximum 500 characters)
    if let Some(ref description) = new_role.description {
        if description.len() > 500 {
            return Err(Error::Validation(
                "Role description must be less than 500 characters".to_string(),
            ));
        }
    }

    // Create the role
    let role = roles::create_role(conn, new_role).await?;

    Ok(role)
}

/// Gets a role by ID with workspace validation
pub async fn get_role_by_id(conn: &mut DbConn, id: Uuid) -> Result<Role> {
    let role = roles::get_role_by_id(conn, id).await?;
    Ok(role)
}

/// Lists all roles in a workspace
pub async fn list_workspace_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>> {
    let roles = roles::list_roles_by_workspace(conn, workspace_id).await?;
    Ok(roles)
}

/// Updates an existing role with validation
pub async fn update_role(conn: &mut DbConn, id: Uuid, update_role: UpdateRole) -> Result<Role> {
    // Get the existing role first
    let existing_role = roles::get_role_by_id(conn, id).await?;

    // Validate new name if provided
    if let Some(ref name) = update_role.name {
        if name.trim().is_empty() {
            return Err(Error::Validation("Role name cannot be empty".to_string()));
        }

        if name.len() > 100 {
            return Err(Error::Validation(
                "Role name must be less than 100 characters".to_string(),
            ));
        }

        // Check if another role with the same name already exists in the workspace
        let duplicate_role = roles::get_role_by_workspace_and_name(
            conn,
            existing_role.workspace_id,
            name,
        )
        .await?;

        if let Some(duplicate) = duplicate_role {
            if duplicate.id != id {
                return Err(Error::Validation(format!(
                    "Role '{}' already exists in this workspace",
                    name
                )));
            }
        }
    }

    // Validate description length if provided
    if let Some(ref description) = update_role.description {
        if description.len() > 500 {
            return Err(Error::Validation(
                "Role description must be less than 500 characters".to_string(),
            ));
        }
    }

    // Update the role
    let updated_role = roles::update_role(conn, id, update_role).await?;

    Ok(updated_role)
}

/// Deletes a role by ID
pub async fn delete_role(conn: &mut DbConn, id: Uuid) -> Result<u64> {
    // Check if the role exists
    let role = roles::get_role_by_id_optional(conn, id).await?;

    if role.is_none() {
        return Err(Error::NotFound("Role not found".to_string()));
    }

    // Delete the role
    let rows_affected = roles::delete_role(conn, id).await?;

    if rows_affected == 0 {
        return Err(Error::NotFound("Role not found".to_string()));
    }

    Ok(rows_affected)
}

/// Validates that a role belongs to a specific workspace
pub async fn validate_role_in_workspace(
    conn: &mut DbConn,
    role_id: Uuid,
    workspace_id: Uuid,
) -> Result<bool> {
    let role = roles::get_role_by_id(conn, role_id).await?;
    Ok(role.workspace_id == workspace_id)
}