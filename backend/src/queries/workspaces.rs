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
        INSERT INTO workspaces (name, owner_id, ai_provider_override)
        VALUES ($1, $2, $3)
        RETURNING id, name, owner_id, NULL as "role_name?", ai_provider_override, created_at, updated_at
        "#,
        new_workspace.name,
        new_workspace.owner_id,
        new_workspace.ai_provider_override
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
        SELECT id, name, owner_id, NULL as "role_name?", ai_provider_override, created_at, updated_at
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
        SELECT id, name, owner_id, NULL as "role_name?", ai_provider_override, created_at, updated_at
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
        SELECT id, name, owner_id, NULL as "role_name?", ai_provider_override, created_at, updated_at
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
        SELECT id, name, owner_id, NULL as "role_name?", ai_provider_override, created_at, updated_at
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
    // Handle the nested Option for ai_provider_override
    // - None: don't update the field
    // - Some(None): set to NULL
    // - Some(Some(value)): set to value
    let workspace = if update_workspace.ai_provider_override.is_some() {
        sqlx::query_as!(
            Workspace,
            r#"
            UPDATE workspaces
            SET name = COALESCE($1, name),
                owner_id = COALESCE($2, owner_id),
                ai_provider_override = $3,
                updated_at = now()
            WHERE id = $4
            RETURNING id, name, owner_id, NULL as "role_name?", ai_provider_override, created_at, updated_at
            "#,
            update_workspace.name,
            update_workspace.owner_id,
            update_workspace.ai_provider_override.flatten(),
            id
        )
        .fetch_one(conn)
        .await
        .map_err(Error::Sqlx)?
    } else {
        sqlx::query_as!(
            Workspace,
            r#"
            UPDATE workspaces
            SET name = COALESCE($1, name),
                owner_id = COALESCE($2, owner_id),
                updated_at = now()
            WHERE id = $3
            RETURNING id, name, owner_id, NULL as "role_name?", ai_provider_override, created_at, updated_at
            "#,
            update_workspace.name,
            update_workspace.owner_id,
            id
        )
        .fetch_one(conn)
        .await
        .map_err(Error::Sqlx)?
    };

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

/// Gets all workspaces where user is owner or member in a single optimized query.
///
/// This uses a single query with LEFT JOIN to fetch workspaces where the user
/// is the owner OR a member, eliminating the N+1 query problem.
///
/// # Performance
/// - Before: N+1 queries (1 for owned + 1 for member IDs + N for each workspace)
/// - After: 1 query with LEFT JOIN
pub async fn get_workspaces_by_user_membership(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<Vec<Workspace>> {
    let workspaces = sqlx::query_as!(
        Workspace,
        r#"
        SELECT DISTINCT
            w.id,
            w.name,
            w.owner_id,
            CASE WHEN r.id IS NOT NULL THEN r.name ELSE NULL END as "role_name?",
            w.ai_provider_override,
            w.created_at,
            w.updated_at
        FROM workspaces w
        LEFT JOIN workspace_members wm ON w.id = wm.workspace_id AND wm.user_id = $1
        LEFT JOIN roles r ON wm.role_id = r.id
        WHERE w.owner_id = $1 OR wm.user_id = $1
        ORDER BY w.created_at DESC
        "#,
        user_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(workspaces)
}