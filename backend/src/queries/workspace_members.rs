use crate::{
    error::{Error, Result},
    models::workspace_members::{NewWorkspaceMember, UpdateWorkspaceMember, WorkspaceMember},
};
use uuid::Uuid;

use crate::DbConn;

/// Creates a new workspace member in the database.
pub async fn create_workspace_member(conn: &mut DbConn, new_member: NewWorkspaceMember) -> Result<WorkspaceMember> {
    let member = sqlx::query_as!(
        WorkspaceMember,
        r#"
        INSERT INTO workspace_members (workspace_id, user_id, role_id)
        VALUES ($1, $2, $3)
        RETURNING workspace_id, user_id, role_id
        "#,
        new_member.workspace_id,
        new_member.user_id,
        new_member.role_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(member)
}

/// Gets a single workspace member by workspace ID and user ID. Expects the member to exist.
pub async fn get_workspace_member(conn: &mut DbConn, workspace_id: Uuid, user_id: Uuid) -> Result<WorkspaceMember> {
    let member = sqlx::query_as!(
        WorkspaceMember,
        r#"
        SELECT workspace_id, user_id, role_id
        FROM workspace_members
        WHERE workspace_id = $1 AND user_id = $2
        "#,
        workspace_id,
        user_id,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(member)
}

/// Gets a single workspace member by workspace ID and user ID. The member may not exist.
pub async fn get_workspace_member_optional(conn: &mut DbConn, workspace_id: Uuid, user_id: Uuid) -> Result<Option<WorkspaceMember>> {
    let member = sqlx::query_as!(
        WorkspaceMember,
        r#"
        SELECT workspace_id, user_id, role_id
        FROM workspace_members
        WHERE workspace_id = $1 AND user_id = $2
        "#,
        workspace_id,
        user_id
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(member)
}

/// Lists all members in a specific workspace.
pub async fn list_workspace_members(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<WorkspaceMember>> {
    let members = sqlx::query_as!(
        WorkspaceMember,
        r#"
        SELECT workspace_id, user_id, role_id
        FROM workspace_members
        WHERE workspace_id = $1
        ORDER BY user_id ASC
        "#,
        workspace_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(members)
}

/// Lists all workspaces that a user is a member of.
pub async fn list_user_workspaces(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<WorkspaceMember>> {
    let memberships = sqlx::query_as!(
        WorkspaceMember,
        r#"
        SELECT workspace_id, user_id, role_id
        FROM workspace_members
        WHERE user_id = $1
        ORDER BY workspace_id ASC
        "#,
        user_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(memberships)
}

/// Lists all workspace members in the database.
pub async fn list_workspace_members_all(conn: &mut DbConn) -> Result<Vec<WorkspaceMember>> {
    let members = sqlx::query_as!(
        WorkspaceMember,
        r#"
        SELECT workspace_id, user_id, role_id
        FROM workspace_members
        ORDER BY workspace_id, user_id ASC
        "#,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(members)
}

/// Updates an existing workspace member's role.
pub async fn update_workspace_member(conn: &mut DbConn, workspace_id: Uuid, user_id: Uuid, update_member: UpdateWorkspaceMember) -> Result<WorkspaceMember> {
    let member = sqlx::query_as!(
        WorkspaceMember,
        r#"
        UPDATE workspace_members
        SET role_id = $1
        WHERE workspace_id = $2 AND user_id = $3
        RETURNING workspace_id, user_id, role_id
        "#,
        update_member.role_id,
        workspace_id,
        user_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(member)
}

/// Deletes a workspace member by workspace ID and user ID.
pub async fn delete_workspace_member(conn: &mut DbConn, workspace_id: Uuid, user_id: Uuid) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM workspace_members
        WHERE workspace_id = $1 AND user_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}

/// Deletes all members in a workspace.
pub async fn delete_workspace_members_by_workspace(conn: &mut DbConn, workspace_id: Uuid) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM workspace_members
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

/// Deletes all workspace memberships for a user.
pub async fn delete_workspace_members_by_user(conn: &mut DbConn, user_id: Uuid) -> Result<u64> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM workspace_members
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?
    .rows_affected();

    Ok(rows_affected)
}

/// Checks if a user is a member of a workspace.
pub async fn is_workspace_member(conn: &mut DbConn, workspace_id: Uuid, user_id: Uuid) -> Result<bool> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!"
        FROM workspace_members
        WHERE workspace_id = $1 AND user_id = $2
        "#,
        workspace_id,
        user_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(count > 0)
}