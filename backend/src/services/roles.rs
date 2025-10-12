use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::roles::{NewRole, Role, DEFAULT_ROLES, descriptions},
    queries::roles,
};
use uuid::Uuid;

/// Creates default roles for a workspace (admin, editor, viewer)
pub async fn create_default_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>> {
    let mut created_roles = Vec::new();

    // Create default roles using constants
    for role_name in DEFAULT_ROLES {
        let new_role = NewRole {
            workspace_id,
            name: role_name.to_string(),
            description: Some(descriptions::for_role(role_name).to_string()),
        };

        let role = create_single_role(conn, new_role).await?;
        created_roles.push(role);
    }

    Ok(created_roles)
}

/// Creates a single custom role (kept for flexibility but simplified)
pub async fn create_single_role(conn: &mut DbConn, new_role: NewRole) -> Result<Role> {
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
pub async fn get_role(conn: &mut DbConn, id: Uuid) -> Result<Role> {
    let role = roles::get_role_by_id(conn, id).await?;
    Ok(role)
}

/// Lists all roles in a workspace
pub async fn list_workspace_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>> {
    let roles = roles::list_roles_by_workspace(conn, workspace_id).await?;
    Ok(roles)
}

/// Gets a role by name in a workspace
pub async fn get_role_by_name(conn: &mut DbConn, workspace_id: Uuid, role_name: &str) -> Result<Role> {
    let role = roles::get_role_by_workspace_and_name(conn, workspace_id, role_name).await?;
    match role {
        Some(role) => Ok(role),
        None => Err(Error::NotFound(format!(
            "Role '{}' not found in workspace",
            role_name
        ))),
    }
}