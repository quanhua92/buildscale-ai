use crate::{
    error::{Error, Result},
    models::invitations::{NewWorkspaceInvitation, UpdateWorkspaceInvitation, WorkspaceInvitation},
    DbConn,
};

use uuid::Uuid;

/// Creates a new workspace invitation in database.
pub async fn create_invitation(conn: &mut DbConn, new_invitation: NewWorkspaceInvitation) -> Result<WorkspaceInvitation> {
    let invitation = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        INSERT INTO workspace_invitations (workspace_id, invited_email, invited_by, role_id, invitation_token, status, expires_at)
        VALUES ($1, $2, $3, $4, $5, 'pending', $6)
        RETURNING id, workspace_id, invited_email, invited_by, role_id, invitation_token,
                 status, expires_at, accepted_at, created_at, updated_at
        "#,
        new_invitation.workspace_id,
        new_invitation.invited_email,
        new_invitation.invited_by,
        new_invitation.role_id,
        new_invitation.invitation_token,
        new_invitation.expires_at
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitation)
}

/// Gets a workspace invitation by its ID. Returns an error if not found.
pub async fn get_invitation_by_id(conn: &mut DbConn, id: Uuid) -> Result<WorkspaceInvitation> {
    let invitation = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        SELECT id, workspace_id, invited_email, invited_by, role_id, invitation_token,
               status, expires_at, accepted_at, created_at, updated_at
        FROM workspace_invitations
        WHERE id = $1
        "#,
        id,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitation)
}

/// Gets a workspace invitation by its ID. Returns None if not found.
pub async fn get_invitation_by_id_optional(conn: &mut DbConn, id: Uuid) -> Result<Option<WorkspaceInvitation>> {
    let invitation = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        SELECT id, workspace_id, invited_email, invited_by, role_id, invitation_token,
               status, expires_at, accepted_at, created_at, updated_at
        FROM workspace_invitations
        WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitation)
}

/// Gets a workspace invitation by its invitation token. Returns an error if not found.
pub async fn get_invitation_by_token(conn: &mut DbConn, token: &str) -> Result<WorkspaceInvitation> {
    let invitation = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        SELECT id, workspace_id, invited_email, invited_by, role_id, invitation_token,
               status, expires_at, accepted_at, created_at, updated_at
        FROM workspace_invitations
        WHERE invitation_token = $1
        "#,
        token,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitation)
}

/// Gets all invitations for a specific workspace.
pub async fn list_invitations_by_workspace(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<WorkspaceInvitation>> {
    let invitations = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        SELECT id, workspace_id, invited_email, invited_by, role_id, invitation_token,
               status, expires_at, accepted_at, created_at, updated_at
        FROM workspace_invitations
        WHERE workspace_id = $1
        ORDER BY created_at DESC
        "#,
        workspace_id,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitations)
}

/// Gets all invitations for a specific email address across all workspaces.
pub async fn list_invitations_by_email(conn: &mut DbConn, email: &str) -> Result<Vec<WorkspaceInvitation>> {
    let invitations = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        SELECT id, workspace_id, invited_email, invited_by, role_id, invitation_token,
               status, expires_at, accepted_at, created_at, updated_at
        FROM workspace_invitations
        WHERE invited_email = $1
        ORDER BY created_at DESC
        "#,
        email,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitations)
}

/// Gets all invitations created by a specific user.
pub async fn list_invitations_by_inviter(conn: &mut DbConn, inviter_id: Uuid) -> Result<Vec<WorkspaceInvitation>> {
    let invitations = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        SELECT id, workspace_id, invited_email, invited_by, role_id, invitation_token,
               status, expires_at, accepted_at, created_at, updated_at
        FROM workspace_invitations
        WHERE invited_by = $1
        ORDER BY created_at DESC
        "#,
        inviter_id,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitations)
}


/// Updates an existing invitation.
pub async fn update_invitation(
    conn: &mut DbConn,
    id: Uuid,
    update_invitation: UpdateWorkspaceInvitation,
) -> Result<WorkspaceInvitation> {
    let invitation = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        UPDATE workspace_invitations
        SET status = COALESCE($1, status),
            expires_at = COALESCE($2, expires_at),
            accepted_at = $3,
            updated_at = NOW()
        WHERE id = $4
        RETURNING id, workspace_id, invited_email, invited_by, role_id, invitation_token,
                 status, expires_at, accepted_at, created_at, updated_at
        "#,
        update_invitation.status,
        update_invitation.expires_at,
        update_invitation.accepted_at,
        id,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitation)
}

/// Updates an invitation status by its token.
pub async fn update_invitation_status_by_token(
    conn: &mut DbConn,
    token: &str,
    status: String,
    accepted_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<WorkspaceInvitation> {
    let invitation = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        UPDATE workspace_invitations
        SET status = $1,
            accepted_at = $2,
            updated_at = NOW()
        WHERE invitation_token = $3
        RETURNING id, workspace_id, invited_email, invited_by, role_id, invitation_token,
                 status, expires_at, accepted_at, created_at, updated_at
        "#,
        status,
        accepted_at,
        token,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitation)
}

/// Deletes a workspace invitation by its ID.
pub async fn delete_invitation(conn: &mut DbConn, id: Uuid) -> Result<u64> {
      let result = sqlx::query!(
        r#"
        DELETE FROM workspace_invitations
        WHERE id = $1
        "#,
        id,
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.rows_affected())
}

/// Deletes a workspace invitation by its token.
pub async fn delete_invitation_by_token(conn: &mut DbConn, token: &str) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        DELETE FROM workspace_invitations
        WHERE invitation_token = $1
        "#,
        token,
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.rows_affected())
}

/// Deletes all invitations for a specific workspace.
pub async fn delete_invitations_by_workspace(conn: &mut DbConn, workspace_id: Uuid) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        DELETE FROM workspace_invitations
        WHERE workspace_id = $1
        "#,
        workspace_id,
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.rows_affected())
}

/// Deletes all invitations that have expired.
pub async fn delete_expired_invitations(conn: &mut DbConn) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        DELETE FROM workspace_invitations
        WHERE expires_at < NOW() AND status != 'accepted'
        "#,
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.rows_affected())
}

/// Checks if a pending invitation exists for a workspace and email combination.
pub async fn check_existing_pending_invitation(
    conn: &mut DbConn,
    workspace_id: Uuid,
    email: &str,
) -> Result<bool> {
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM workspace_invitations
            WHERE workspace_id = $1 AND invited_email = $2 AND status = 'pending'
        )
        "#,
        workspace_id,
        email,
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(exists.unwrap_or(false))
}

/// Counts invitations by status for a workspace.
pub async fn count_invitations_by_status(
    conn: &mut DbConn,
    workspace_id: Uuid,
) -> Result<Vec<(String, i64)>> {
    let rows = sqlx::query!(
        r#"
        SELECT status, COUNT(*) as count
        FROM workspace_invitations
        WHERE workspace_id = $1
        GROUP BY status
        ORDER BY count DESC
        "#,
        workspace_id,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    let result = rows
        .into_iter()
        .map(|row| (row.status, row.count.unwrap_or(0)))
        .collect();

    Ok(result)
}

/// Gets recent invitations for a workspace (last N days).
pub async fn get_recent_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
    days: i64,
) -> Result<Vec<WorkspaceInvitation>> {
    let invitations = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        SELECT id, workspace_id, invited_email, invited_by, role_id, invitation_token,
               status, expires_at, accepted_at, created_at, updated_at
        FROM workspace_invitations
        WHERE workspace_id = $1 AND created_at >= NOW() - INTERVAL '1 day' * $2
        ORDER BY created_at DESC
        "#,
        workspace_id,
        days as f64,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitations)
}

/// Gets invitations that are about to expire (within next N hours).
pub async fn get_invitations_expiring_soon(
    conn: &mut DbConn,
    hours: i64,
) -> Result<Vec<WorkspaceInvitation>> {
    let invitations = sqlx::query_as!(
        WorkspaceInvitation,
        r#"
        SELECT id, workspace_id, invited_email, invited_by, role_id, invitation_token,
               status, expires_at, accepted_at, created_at, updated_at
        FROM workspace_invitations
        WHERE status = 'pending'
              AND expires_at >= NOW()
              AND expires_at <= NOW() + INTERVAL '1 hour' * $1
        ORDER BY expires_at ASC
        "#,
        hours as f64,
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(invitations)
}