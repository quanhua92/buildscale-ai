use crate::{
    error::{Error, Result},
    models::workspaces::{NewWorkspace, UpdateWorkspace, Workspace},
};
use uuid::Uuid;

use crate::DbConn;

/// Creates a new workspace in the database.
pub async fn create_workspace(conn: &mut DbConn, new_workspace: NewWorkspace) -> Result<Workspace> {
    let workspace = sqlx::query_as!(
        Workspace,
        r#"
        INSERT INTO workspaces (name, owner_id)
        VALUES ($1, $2)
        RETURNING id, name, owner_id, created_at, updated_at
        "#,
        new_workspace.name,
        new_workspace.owner_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(workspace)
}

/// Gets a single workspace by their ID. Expects the workspace to exist.
pub async fn get_workspace_by_id(conn: &mut DbConn, id: Uuid) -> Result<Workspace> {
    let workspace = sqlx::query_as!(
        Workspace,
        r#"
        SELECT id, name, owner_id, created_at, updated_at
        FROM workspaces
        WHERE id = $1
        "#,
        id,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(workspace)
}

/// Gets a single workspace by their ID. The workspace may not exist.
pub async fn get_workspace_by_id_optional(conn: &mut DbConn, id: Uuid) -> Result<Option<Workspace>> {
    let workspace = sqlx::query_as!(
        Workspace,
        r#"
        SELECT id, name, owner_id, created_at, updated_at
        FROM workspaces
        WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(workspace)
}

/// Gets a single workspace by owner ID. The workspace may not exist.
pub async fn get_workspaces_by_owner(conn: &mut DbConn, owner_id: Uuid) -> Result<Vec<Workspace>> {
    let workspaces = sqlx::query_as!(
        Workspace,
        r#"
        SELECT id, name, owner_id, created_at, updated_at
        FROM workspaces
        WHERE owner_id = $1
        ORDER BY created_at DESC
        "#,
        owner_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(workspaces)
}

/// Lists all workspaces in the database.
pub async fn list_workspaces(conn: &mut DbConn) -> Result<Vec<Workspace>> {
    let workspaces = sqlx::query_as!(
        Workspace,
        r#"
        SELECT id, name, owner_id, created_at, updated_at
        FROM workspaces
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(workspaces)
}

/// Updates an existing workspace's details.
pub async fn update_workspace(conn: &mut DbConn, id: Uuid, update_workspace: UpdateWorkspace) -> Result<Workspace> {
    let workspace = sqlx::query_as!(
        Workspace,
        r#"
        UPDATE workspaces
        SET name = COALESCE($1, name),
            owner_id = COALESCE($2, owner_id),
            updated_at = now()
        WHERE id = $3
        RETURNING id, name, owner_id, created_at, updated_at
        "#,
        update_workspace.name,
        update_workspace.owner_id,
        id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(workspace)
}

/// Deletes a workspace by their ID.
pub async fn delete_workspace(conn: &mut DbConn, id: Uuid) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM workspaces
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

/// Checks if a user is the owner of a workspace.
pub async fn is_workspace_owner(conn: &mut DbConn, workspace_id: Uuid, user_id: Uuid) -> Result<bool> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM workspaces
        WHERE id = $1 AND owner_id = $2
        "#,
        workspace_id,
        user_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(count > 0)
}