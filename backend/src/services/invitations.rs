use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::{
        invitations::{
            WorkspaceInvitation, NewWorkspaceInvitation, UpdateWorkspaceInvitation,
            CreateInvitationRequest, CreateInvitationResponse, AcceptInvitationRequest,
            AcceptInvitationResponse, RevokeInvitationRequest, InvitationStatus,
            InvitationValidator, InvitationUtils, DEFAULT_INVITATION_EXPIRATION_HOURS,
        },
        workspace_members::NewWorkspaceMember,
        permissions::{workspace_permissions, member_permissions},
    },
    queries::{
        invitations, workspaces, roles, workspace_members, users,
    },
    services::workspace_members::validate_workspace_permission,
};
use uuid::Uuid;

/// Creates a new workspace invitation
pub async fn create_invitation(
    conn: &mut DbConn,
    request: CreateInvitationRequest,
    inviter_id: Uuid,
) -> Result<CreateInvitationResponse> {
    // Validate email format
    InvitationValidator::validate_email(&request.invited_email)
        .map_err(|e| Error::Validation(e))?;

    // Check if inviter has permission to invite members
    validate_workspace_permission(
        conn,
        request.workspace_id,
        inviter_id,
        workspace_permissions::INVITE_MEMBERS,
    ).await?;

    // Validate workspace exists
    let _workspace = workspaces::get_workspace_by_id(conn, request.workspace_id).await?;

    // Get the role for the invitation
    let role = roles::get_role_by_workspace_and_name(
        conn,
        request.workspace_id,
        &request.role_name.to_lowercase(),
    ).await?
    .ok_or_else(|| Error::NotFound(format!(
        "Role '{}' not found in workspace",
        request.role_name
    )))?;

    // Validate the role belongs to the workspace
    if role.workspace_id != request.workspace_id {
        return Err(Error::Validation(
            "Role does not belong to the specified workspace".to_string(),
        ));
    }

    // Check if user is already a member of the workspace
    let user_opt = users::get_user_by_email(conn, &request.invited_email).await?;
    if let Some(user) = user_opt {
        let existing_member = workspace_members::is_workspace_member(
            conn,
            request.workspace_id,
            user.id,
        ).await?;

        if existing_member {
            return Err(Error::Conflict(
                "User is already a member of this workspace".to_string(),
            ));
        }
    }

    // Check if there's already a pending invitation for this workspace and email
    let existing_pending = invitations::check_existing_pending_invitation(
        conn,
        request.workspace_id,
        &request.invited_email,
    ).await?;

    if existing_pending {
        return Err(Error::Conflict(
            "A pending invitation already exists for this email address".to_string(),
        ));
    }

    // Calculate expiration time
    let hours = request.expires_in_hours.unwrap_or(DEFAULT_INVITATION_EXPIRATION_HOURS);
    InvitationValidator::validate_expiration_hours(hours)
        .map_err(|e| Error::Validation(e))?;

    let expires_at = InvitationUtils::calculate_expiration(hours);

    // Generate invitation token
    let invitation_token = InvitationUtils::generate_invitation_token();

    // Create the invitation
    let new_invitation = NewWorkspaceInvitation {
        workspace_id: request.workspace_id,
        invited_email: request.invited_email.to_lowercase(), // Normalize email
        invited_by: inviter_id,
        role_id: role.id,
        invitation_token,
        expires_at,
    };

    let invitation = invitations::create_invitation(conn, new_invitation).await?;

    // Generate invitation URL (you might want to configure base URL)
    let invitation_url = InvitationUtils::generate_invitation_url(
        "https://your-domain.com", // TODO: Make this configurable
        &invitation.invitation_token,
    );

    Ok(CreateInvitationResponse {
        invitation,
        invitation_url,
    })
}

/// Gets an invitation by its token
pub async fn get_invitation_by_token(
    conn: &mut DbConn,
    token: &str,
) -> Result<WorkspaceInvitation> {
    // Validate token format
    InvitationValidator::validate_invitation_token(token)
        .map_err(|e| Error::Validation(e))?;

    invitations::get_invitation_by_token(conn, token).await
}

/// Lists all invitations for a workspace
pub async fn list_workspace_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
    requester_id: Uuid,
) -> Result<Vec<WorkspaceInvitation>> {
    // Check if requester has permission to view members
    validate_workspace_permission(
        conn,
        workspace_id,
        requester_id,
        member_permissions::VIEW_MEMBERS,
    ).await?;

    // Validate workspace exists
    let _workspace = workspaces::get_workspace_by_id(conn, workspace_id).await?;

    invitations::list_invitations_by_workspace(conn, workspace_id).await
}

/// Lists invitations sent by a specific user
pub async fn list_user_sent_invitations(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<Vec<WorkspaceInvitation>> {
    invitations::list_invitations_by_inviter(conn, user_id).await
}

/// Lists invitations for a specific email address
pub async fn list_email_invitations(
    conn: &mut DbConn,
    email: &str,
) -> Result<Vec<WorkspaceInvitation>> {
    // Validate email format
    InvitationValidator::validate_email(email)
        .map_err(|e| Error::Validation(e))?;

    let normalized_email = email.to_lowercase();
    invitations::list_invitations_by_email(conn, &normalized_email).await
}

/// Accepts a workspace invitation
pub async fn accept_invitation(
    conn: &mut DbConn,
    request: AcceptInvitationRequest,
    user_id: Uuid,
) -> Result<AcceptInvitationResponse> {
    // Validate token format
    InvitationValidator::validate_invitation_token(&request.invitation_token)
        .map_err(|e| Error::Validation(e))?;

    // Get the invitation
    let mut invitation = invitations::get_invitation_by_token(
        conn,
        &request.invitation_token,
    ).await?;

    // Check if invitation can be accepted
    let status_enum = invitation.status_enum();
    if !InvitationValidator::can_accept(&status_enum, invitation.expires_at) {
        let reason = match status_enum {
            InvitationStatus::Accepted => "Invitation has already been accepted",
            InvitationStatus::Revoked => "Invitation has been revoked",
            InvitationStatus::Expired => "Invitation has expired",
            InvitationStatus::Pending => {
                if InvitationValidator::is_expired(invitation.expires_at) {
                    "Invitation has expired"
                } else {
                    "Invitation cannot be accepted in current state"
                }
            }
        };
        return Err(Error::Validation(reason.to_string()));
    }

    // Verify that the accepting user's email matches the invitation
    let user = users::get_user_by_id(conn, user_id).await?
        .ok_or_else(|| Error::NotFound("User not found".to_string()))?;
    if user.email.to_lowercase() != invitation.invited_email.to_lowercase() {
        return Err(Error::Forbidden(
            "This invitation was sent to a different email address".to_string(),
        ));
    }

    // Check if user is already a member (shouldn't happen, but let's be safe)
    let is_already_member = workspace_members::is_workspace_member(
        conn,
        invitation.workspace_id,
        user_id,
    ).await?;

    if is_already_member {
        // Update invitation to accepted anyway for consistency
        let _ = invitations::update_invitation_status_by_token(
            conn,
            &request.invitation_token,
            InvitationStatus::Accepted.to_string(),
            Some(chrono::Utc::now()),
        ).await?;

        return Err(Error::Conflict(
            "You are already a member of this workspace".to_string(),
        ));
    }

    // Create workspace member
    let new_member = NewWorkspaceMember {
        workspace_id: invitation.workspace_id,
        user_id,
        role_id: invitation.role_id,
    };

    let workspace_member = workspace_members::create_workspace_member(conn, new_member).await?;

    // Update invitation status to accepted
    invitation = invitations::update_invitation_status_by_token(
        conn,
        &request.invitation_token,
        InvitationStatus::Accepted.to_string(),
        Some(chrono::Utc::now()),
    ).await?;

    Ok(AcceptInvitationResponse {
        invitation,
        workspace_member,
    })
}

/// Revokes a workspace invitation
pub async fn revoke_invitation(
    conn: &mut DbConn,
    request: RevokeInvitationRequest,
    revoker_id: Uuid,
) -> Result<WorkspaceInvitation> {
    // Get the invitation
    let invitation = invitations::get_invitation_by_id(conn, request.invitation_id).await?;

    // Check if revoker has permission to manage members
    validate_workspace_permission(
        conn,
        invitation.workspace_id,
        revoker_id,
        workspace_permissions::MANAGE_MEMBERS,
    ).await?;

    // Check if invitation can be revoked
    let status_enum = invitation.status_enum();
    if !InvitationValidator::can_revoke(&status_enum) {
        return Err(Error::Validation(
            "Invitation cannot be revoked in current state".to_string(),
        ));
    }

    // Update invitation status to revoked
    invitations::update_invitation(
        conn,
        invitation.id,
        UpdateWorkspaceInvitation {
            status: Some(InvitationStatus::Revoked.to_string()),
            expires_at: None,
            accepted_at: None,
        },
    ).await
}

/// Deletes a workspace invitation (soft delete via revoke)
pub async fn delete_invitation(
    conn: &mut DbConn,
    invitation_id: Uuid,
    deleter_id: Uuid,
) -> Result<u64> {
    // Get the invitation first to check permissions
    let invitation = invitations::get_invitation_by_id(conn, invitation_id).await?;

    // Check if deleter has permission to manage members
    validate_workspace_permission(
        conn,
        invitation.workspace_id,
        deleter_id,
        workspace_permissions::MANAGE_MEMBERS,
    ).await?;

    // Delete the invitation
    invitations::delete_invitation(conn, invitation_id).await
}

/// Cleans up expired invitations
pub async fn cleanup_expired_invitations(conn: &mut DbConn) -> Result<u64> {
    invitations::delete_expired_invitations(conn).await
}

/// Gets invitations that are about to expire (for notification purposes)
pub async fn get_invitations_expiring_soon(
    conn: &mut DbConn,
    hours: i32,
) -> Result<Vec<WorkspaceInvitation>> {
    invitations::get_invitations_expiring_soon(conn, hours.into()).await
}

/// Gets invitation statistics for a workspace
pub async fn get_workspace_invitation_stats(
    conn: &mut DbConn,
    workspace_id: Uuid,
    requester_id: Uuid,
) -> Result<Vec<(String, i64)>> {
    // Check if requester has permission to view members
    validate_workspace_permission(
        conn,
        workspace_id,
        requester_id,
        member_permissions::VIEW_MEMBERS,
    ).await?;

    invitations::count_invitations_by_status(conn, workspace_id).await
}

/// Resends an invitation (creates a new one with the same details)
pub async fn resend_invitation(
    conn: &mut DbConn,
    invitation_id: Uuid,
    resender_id: Uuid,
    expires_in_hours: Option<i64>,
) -> Result<CreateInvitationResponse> {
    // Get the original invitation
    let original_invitation = invitations::get_invitation_by_id(conn, invitation_id).await?;

    // Check if resender has permission to invite members
    validate_workspace_permission(
        conn,
        original_invitation.workspace_id,
        resender_id,
        workspace_permissions::INVITE_MEMBERS,
    ).await?;

    // Get the role details
    let role = roles::get_role_by_id(conn, original_invitation.role_id).await?;

    // Create new invitation request
    let resend_request = CreateInvitationRequest {
        workspace_id: original_invitation.workspace_id,
        invited_email: original_invitation.invited_email.clone(),
        role_name: role.name.clone(),
        expires_in_hours,
    };

    create_invitation(conn, resend_request, resender_id).await
}

/// Bulk create invitations (for inviting multiple users at once)
pub async fn bulk_create_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
    emails: Vec<String>,
    role_name: String,
    inviter_id: Uuid,
    expires_in_hours: Option<i64>,
) -> Result<Vec<CreateInvitationResponse>> {
    // Check if inviter has permission to invite members
    validate_workspace_permission(
        conn,
        workspace_id,
        inviter_id,
        workspace_permissions::INVITE_MEMBERS,
    ).await?;

    if emails.is_empty() {
        return Err(Error::Validation("No email addresses provided".to_string()));
    }

    if emails.len() > 100 {
        return Err(Error::Validation("Cannot invite more than 100 users at once".to_string()));
    }

    let mut responses = Vec::new();

    for email in emails {
        let request = CreateInvitationRequest {
            workspace_id,
            invited_email: email,
            role_name: role_name.clone(),
            expires_in_hours,
        };

        match create_invitation(conn, request, inviter_id).await {
            Ok(response) => responses.push(response),
            Err(_) => {
                // Continue processing other emails even if one fails
                // In a production system, you might want to collect errors separately
                continue;
            }
        }
    }

    Ok(responses)
}

#[cfg(test)]
mod tests {
    use super::*;
  
    // Note: Integration tests would need a test database setup
    // These are unit tests for validation logic

    #[test]
    fn test_invitation_validation() {
        assert!(InvitationValidator::validate_email("test@example.com").is_ok());
        assert!(InvitationValidator::validate_email("").is_err());
        assert!(InvitationValidator::validate_email("invalid").is_err());

        assert!(InvitationValidator::validate_expiration_hours(24).is_ok());
        assert!(InvitationValidator::validate_expiration_hours(0).is_err());
        assert!(InvitationValidator::validate_expiration_hours(1000).is_err());

        let valid_token = "018f1234-5678-9abc-def0-123456789abc";
        let invalid_token = "invalid";
        assert!(InvitationValidator::validate_invitation_token(valid_token).is_ok());
        assert!(InvitationValidator::validate_invitation_token(invalid_token).is_err());
    }

    #[test]
    fn test_invitation_utility_functions() {
        let token = InvitationUtils::generate_invitation_token();
        assert_eq!(token.len(), 36);

        let expiration = InvitationUtils::calculate_expiration(24);
        let expected = chrono::Utc::now() + chrono::Duration::hours(24);
        let diff = (expiration - expected).num_minutes().abs();
        assert!(diff <= 1); // Allow for small timing differences

        let default_exp = InvitationUtils::get_default_expiration();
        let expected_default = chrono::Utc::now() + chrono::Duration::hours(DEFAULT_INVITATION_EXPIRATION_HOURS);
        let diff_default = (default_exp - expected_default).num_minutes().abs();
        assert!(diff_default <= 1);
    }

    #[test]
    fn test_invitation_state_logic() {
        let now = chrono::Utc::now();
        let future = now + chrono::Duration::hours(1);
        let past = now - chrono::Duration::hours(1);

        assert!(InvitationValidator::can_accept(
            &InvitationStatus::Pending,
            future
        ));
        assert!(!InvitationValidator::can_accept(
            &InvitationStatus::Accepted,
            future
        ));
        assert!(!InvitationValidator::can_accept(
            &InvitationStatus::Pending,
            past
        ));

        assert!(InvitationValidator::can_revoke(&InvitationStatus::Pending));
        assert!(InvitationValidator::can_revoke(&InvitationStatus::Expired));
        assert!(!InvitationValidator::can_revoke(&InvitationStatus::Accepted));
    }

    #[test]
    fn test_invitation_url_generation() {
        let token = "018f1234-5678-9abc-def0-123456789abc";
        let url = InvitationUtils::generate_invitation_url("https://example.com", token);
        assert_eq!(url, "https://example.com/invitations/018f1234-5678-9abc-def0-123456789abc");

        // Test URL generation with trailing slash in base URL
        let url_with_slash = InvitationUtils::generate_invitation_url("https://example.com/", token);
        assert_eq!(url_with_slash, "https://example.com/invitations/018f1234-5678-9abc-def0-123456789abc");
    }
}