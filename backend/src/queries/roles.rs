use crate::{
    error::{Error, Result},
    models::roles::{NewRole, Role, UpdateRole},
};
use uuid::Uuid;

use crate::DbConn;

/// Creates a new role in the database.
pub async fn create_role(conn: &mut DbConn, new_role: NewRole) -> Result<Role> {
    let role = sqlx::query_as!(
        Role,
        r#"
        INSERT INTO roles (workspace_id, name, description)
        VALUES ($1, $2, $3)
        RETURNING id, workspace_id, name, description
        "#,
        new_role.workspace_id,
        new_role.name,
        new_role.description
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(role)
}

/// Gets a single role by their ID. Expects the role to exist.
pub async fn get_role_by_id(conn: &mut DbConn, id: Uuid) -> Result<Role> {
    let role = sqlx::query_as!(
        Role,
        r#"
        SELECT id, workspace_id, name, description
        FROM roles
        WHERE id = $1
        "#,
        id,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(role)
}

/// Gets a single role by workspace ID and name. The role may not exist.
pub async fn get_role_by_workspace_and_name(conn: &mut DbConn, workspace_id: Uuid, name: &str) -> Result<Option<Role>> {
    let role = sqlx::query_as!(
        Role,
        r#"
        SELECT id, workspace_id, name, description
        FROM roles
        WHERE workspace_id = $1 AND name = $2
        "#,
        workspace_id,
        name
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(role)
}

/// Lists all roles in a specific workspace.
pub async fn list_roles_by_workspace(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>> {
    let roles = sqlx::query_as!(
        Role,
        r#"
        SELECT id, workspace_id, name, description
        FROM roles
        WHERE workspace_id = $1
        ORDER BY name ASC
        "#,
        workspace_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(roles)
}

/// Lists all roles in the database.
pub async fn list_roles(conn: &mut DbConn) -> Result<Vec<Role>> {
    let roles = sqlx::query_as!(
        Role,
        r#"
        SELECT id, workspace_id, name, description
        FROM roles
        ORDER BY workspace_id, name ASC
        "#,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(roles)
}

/// Updates an existing role's details.
pub async fn update_role(conn: &mut DbConn, id: Uuid, update_role: UpdateRole) -> Result<Role> {
    let role = sqlx::query_as!(
        Role,
        r#"
        UPDATE roles
        SET name = COALESCE($1, name),
            description = $2
        WHERE id = $3
        RETURNING id, workspace_id, name, description
        "#,
        update_role.name,
        update_role.description,
        id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(role)
}

/// Deletes a role by their ID.
pub async fn delete_role(conn: &mut DbConn, id: Uuid) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM roles
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}

/// Deletes all roles in a workspace.
pub async fn delete_roles_by_workspace(conn: &mut DbConn, workspace_id: Uuid) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM roles
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}