use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Invitation status constants
pub const INVITATION_STATUS_PENDING: &str = "pending";
pub const INVITATION_STATUS_ACCEPTED: &str = "accepted";
pub const INVITATION_STATUS_EXPIRED: &str = "expired";
pub const INVITATION_STATUS_REVOKED: &str = "revoked";

/// All valid invitation statuses
pub const VALID_INVITATION_STATUSES: &[&str] = &[
    INVITATION_STATUS_PENDING,
    INVITATION_STATUS_ACCEPTED,
    INVITATION_STATUS_EXPIRED,
    INVITATION_STATUS_REVOKED,
];

/// Default invitation expiration period (in hours)
pub const DEFAULT_INVITATION_EXPIRATION_HOURS: i64 = 168; // 7 days

/// Maximum invitation expiration period (in hours)
pub const MAX_INVITATION_EXPIRATION_HOURS: i64 = 720; // 30 days

/// Invitation status enum for type safety
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Expired,
    Revoked,
}


impl InvitationStatus {
    /// Get the string representation of the status
    pub fn as_str(&self) -> &'static str {
        match self {
            InvitationStatus::Pending => INVITATION_STATUS_PENDING,
            InvitationStatus::Accepted => INVITATION_STATUS_ACCEPTED,
            InvitationStatus::Expired => INVITATION_STATUS_EXPIRED,
            InvitationStatus::Revoked => INVITATION_STATUS_REVOKED,
        }
    }

    /// Create an InvitationStatus from a string
    pub fn from_str(status: &str) -> Option<Self> {
        match status {
            INVITATION_STATUS_PENDING => Some(InvitationStatus::Pending),
            INVITATION_STATUS_ACCEPTED => Some(InvitationStatus::Accepted),
            INVITATION_STATUS_EXPIRED => Some(InvitationStatus::Expired),
            INVITATION_STATUS_REVOKED => Some(InvitationStatus::Revoked),
            _ => None,
        }
    }
}

impl AsRef<str> for InvitationStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for InvitationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Workspace invitation entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInvitation {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub invited_email: String,
    pub invited_by: Uuid,
    pub role_id: Uuid,
    pub invitation_token: String,
    pub status: String, // Using String to avoid SQLx complexity
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkspaceInvitation {
    /// Get the status as InvitationStatus enum
    pub fn status_enum(&self) -> InvitationStatus {
        InvitationStatus::from_str(&self.status).unwrap_or(InvitationStatus::Pending)
    }
}

/// New workspace invitation entity for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkspaceInvitation {
    pub workspace_id: Uuid,
    pub invited_email: String,
    pub invited_by: Uuid,
    pub role_id: Uuid,
    pub invitation_token: String,
    pub expires_at: DateTime<Utc>,
}

/// Update workspace invitation entity for modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceInvitation {
    pub status: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
}

/// Request to create a new invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvitationRequest {
    pub workspace_id: Uuid,
    pub invited_email: String,
    pub role_name: String,
    pub expires_in_hours: Option<i64>,
}

/// Response after creating an invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvitationResponse {
    pub invitation: WorkspaceInvitation,
    pub invitation_url: String,
}

/// Request to accept an invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptInvitationRequest {
    pub invitation_token: String,
}

/// Response after accepting an invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptInvitationResponse {
    pub invitation: WorkspaceInvitation,
    pub workspace_member: crate::models::workspace_members::WorkspaceMember,
}

/// Request to revoke an invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeInvitationRequest {
    pub invitation_id: Uuid,
}

/// Summary of invitation information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationSummary {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub workspace_name: Option<String>, // Populated when joining with workspace
    pub invited_email: String,
    pub invited_by: Uuid,
    pub invited_by_name: Option<String>, // Populated when joining with user
    pub role_name: Option<String>, // Populated when joining with role
    pub status: String, // Using String to avoid SQLx complexity
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl InvitationSummary {
    /// Get the status as InvitationStatus enum
    pub fn status_enum(&self) -> InvitationStatus {
        InvitationStatus::from_str(&self.status).unwrap_or(InvitationStatus::Pending)
    }
}

/// Invitation validation utilities
pub struct InvitationValidator;

impl InvitationValidator {
    /// Validate that an email is properly formatted
    pub fn validate_email(email: &str) -> Result<(), String> {
        if email.trim().is_empty() {
            return Err("Email cannot be empty".to_string());
        }

        // Basic email validation
        if !email.contains('@') || !email.contains('.') {
            return Err("Invalid email format".to_string());
        }

        if email.len() > 255 {
            return Err("Email address too long (max 255 characters)".to_string());
        }

        Ok(())
    }

    /// Validate that an invitation token is properly formatted
    pub fn validate_invitation_token(token: &str) -> Result<(), String> {
        if token.trim().is_empty() {
            return Err("Invitation token cannot be empty".to_string());
        }

        if token.len() != 36 || !token.starts_with("018") {
            // UUID v7 validation - basic check
            return Err("Invalid invitation token format".to_string());
        }

        Ok(())
    }

    /// Validate expiration hours
    pub fn validate_expiration_hours(hours: i64) -> Result<(), String> {
        if hours < 1 {
            return Err("Expiration must be at least 1 hour".to_string());
        }

        if hours > MAX_INVITATION_EXPIRATION_HOURS {
            return Err(format!(
                "Expiration cannot exceed {} hours ({} days)",
                MAX_INVITATION_EXPIRATION_HOURS,
                MAX_INVITATION_EXPIRATION_HOURS / 24
            ));
        }

        Ok(())
    }

    /// Check if an invitation is expired
    pub fn is_expired(expires_at: DateTime<Utc>) -> bool {
        expires_at <= Utc::now()
    }

    /// Check if an invitation status allows acceptance
    pub fn can_accept(status: &InvitationStatus, expires_at: DateTime<Utc>) -> bool {
        status == &InvitationStatus::Pending && !Self::is_expired(expires_at)
    }

    /// Check if an invitation status allows revocation
    pub fn can_revoke(status: &InvitationStatus) -> bool {
        matches!(status, InvitationStatus::Pending | InvitationStatus::Expired)
    }
}

/// Invitation utilities
pub struct InvitationUtils;

impl InvitationUtils {
    /// Generate a secure invitation token
    pub fn generate_invitation_token() -> String {
        // Use UUID v7 for time-based ordering and uniqueness
        Uuid::now_v7().to_string()
    }

    /// Calculate expiration time from hours
    pub fn calculate_expiration(hours: i64) -> DateTime<Utc> {
        Utc::now() + chrono::Duration::hours(hours)
    }

    /// Get default expiration time
    pub fn get_default_expiration() -> DateTime<Utc> {
        Self::calculate_expiration(DEFAULT_INVITATION_EXPIRATION_HOURS)
    }

    /// Generate invitation URL
    pub fn generate_invitation_url(base_url: &str, token: &str) -> String {
        let base_url = base_url.trim_end_matches('/');
        format!("{}/invitations/{}", base_url, token)
    }

    /// Create a default invitation token with proper expiration
    pub fn create_default_invitation_data() -> (String, DateTime<Utc>) {
        let token = Self::generate_invitation_token();
        let expires_at = Self::get_default_expiration();
        (token, expires_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invitation_status_enum() {
        assert_eq!(InvitationStatus::Pending.as_str(), "pending");
        assert_eq!(InvitationStatus::Accepted.as_str(), "accepted");
        assert_eq!(InvitationStatus::Expired.as_str(), "expired");
        assert_eq!(InvitationStatus::Revoked.as_str(), "revoked");

        assert_eq!(
            InvitationStatus::from_str("pending"),
            Some(InvitationStatus::Pending)
        );
        assert_eq!(
            InvitationStatus::from_str("invalid"),
            None
        );
    }

    #[test]
    fn test_email_validation() {
        assert!(InvitationValidator::validate_email("test@example.com").is_ok());
        assert!(InvitationValidator::validate_email("user.name+tag@domain.co.uk").is_ok());

        assert!(InvitationValidator::validate_email("").is_err());
        assert!(InvitationValidator::validate_email("invalid-email").is_err());
        assert!(InvitationValidator::validate_email("just@").is_err());
    }

    #[test]
    fn test_expiration_validation() {
        assert!(InvitationValidator::validate_expiration_hours(1).is_ok());
        assert!(InvitationValidator::validate_expiration_hours(168).is_ok());
        assert!(InvitationValidator::validate_expiration_hours(720).is_ok());

        assert!(InvitationValidator::validate_expiration_hours(0).is_err());
        assert!(InvitationValidator::validate_expiration_hours(-1).is_err());
        assert!(InvitationValidator::validate_expiration_hours(721).is_err());
    }

    #[test]
    fn test_invitation_token_validation() {
        let valid_token = "018f1234-5678-9abc-def0-123456789abc";
        let invalid_token = "invalid-token";

        assert!(InvitationValidator::validate_invitation_token(valid_token).is_ok());
        assert!(InvitationValidator::validate_invitation_token(invalid_token).is_err());
    }

    #[test]
    fn test_invitation_states() {
        let now = Utc::now();
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
        assert!(!InvitationValidator::can_revoke(&InvitationStatus::Revoked));
    }

    #[test]
    fn test_utility_functions() {
        let token = InvitationUtils::generate_invitation_token();
        assert_eq!(token.len(), 36);

        let expiration = InvitationUtils::calculate_expiration(24);
        let expected = Utc::now() + chrono::Duration::hours(24);
        let diff = (expiration - expected).num_minutes().abs();
        assert!(diff <= 1); // Allow for small timing differences

        let url = InvitationUtils::generate_invitation_url("https://example.com", &token);
        assert_eq!(url, format!("https://example.com/invitations/{}", token));
    }

    #[test]
    fn test_default_constants() {
        assert_eq!(DEFAULT_INVITATION_EXPIRATION_HOURS, 168);
        assert_eq!(MAX_INVITATION_EXPIRATION_HOURS, 720);

        let default_exp = InvitationUtils::get_default_expiration();
        let expected = Utc::now() + chrono::Duration::hours(DEFAULT_INVITATION_EXPIRATION_HOURS);
        let diff = (default_exp - expected).num_minutes().abs();
        assert!(diff <= 1);
    }
}