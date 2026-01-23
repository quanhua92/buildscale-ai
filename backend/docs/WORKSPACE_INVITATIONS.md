← [Back to Index](./README.md) | **Related**: [User Management](./USER_WORKSPACE_MANAGEMENT.md), [RBAC](./ROLE_MANAGEMENT.md)

# Workspace Invitation System

Secure token-based invitation system with role assignments for workspace member onboarding.

## Key Features

- **UUID v7 Tokens**: Secure invitation tokens with configurable default expiration
- **Role Assignment**: Direct role assignment on invitation acceptance
- **Permission Validation**: Requires `INVITE_MEMBERS` permission to send invitations
- **State Management**: pending → accepted/expired/revoked lifecycle
- **Bulk Operations**: Support for inviting multiple users efficiently
- **Email Integration**: Case-insensitive email handling with validation

## Core APIs

### Invitation Management
```rust
// Create invitation with role assignment
pub async fn create_invitation(
    conn: &mut DbConn,
    request: CreateInvitationRequest,
    inviter_id: Uuid,
) -> Result<CreateInvitationResponse>

// Accept invitation and create membership
pub async fn accept_invitation(
    conn: &mut DbConn,
    request: AcceptInvitationRequest,
    user_id: Uuid,
) -> Result<AcceptInvitationResponse>

// Revoke pending invitation
pub async fn revoke_invitation(
    conn: &mut DbConn,
    request: RevokeInvitationRequest,
    revoker_id: Uuid,
) -> Result<WorkspaceInvitation>

// Delete invitation (hard delete)
pub async fn delete_invitation(
    conn: &mut DbConn,
    invitation_id: Uuid,
    deleter_id: Uuid,
) -> Result<u64>
```

### Invitation Queries
```rust
// Get invitation by token
pub async fn get_invitation_by_token(
    conn: &mut DbConn,
    token: &str,
) -> Result<WorkspaceInvitation>

// List workspace invitations (requires VIEW_MEMBERS permission)
pub async fn list_workspace_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
    requester_id: Uuid,
) -> Result<Vec<WorkspaceInvitation>>

// List invitations sent by user
pub async fn list_user_sent_invitations(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<Vec<WorkspaceInvitation>>

// List invitations for email address
pub async fn list_email_invitations(
    conn: &mut DbConn,
    email: &str,
) -> Result<Vec<WorkspaceInvitation>>
```

### Advanced Operations
```rust
// Bulk create invitations (up to 100 users)
pub async fn bulk_create_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
    emails: Vec<String>,
    role_name: String,
    inviter_id: Uuid,
    expires_in_hours: Option<i64>,
) -> Result<Vec<CreateInvitationResponse>>

// Resend invitation (creates new with same details)
pub async fn resend_invitation(
    conn: &mut DbConn,
    invitation_id: Uuid,
    resender_id: Uuid,
    expires_in_hours: Option<i64>,
) -> Result<CreateInvitationResponse>

// Cleanup expired invitations
pub async fn cleanup_expired_invitations(conn: &mut DbConn) -> Result<u64>

// Get invitations expiring soon (for notifications)
pub async fn get_invitations_expiring_soon(
    conn: &mut DbConn,
    hours: i32,
) -> Result<Vec<WorkspaceInvitation>>
```

## Data Models

### Core Entities
```rust
pub struct WorkspaceInvitation {
    pub id: Uuid,                    // Primary key (UUID v7)
    pub workspace_id: Uuid,          // Target workspace
    pub invited_email: String,        // Email of invited user
    pub invited_by: Uuid,           // User who sent invitation
    pub role_id: Uuid,              // Role to assign on acceptance
    pub invitation_token: String,    // Secure acceptance token (UUID v7)
    pub status: String,             // pending, accepted, expired, revoked
    pub expires_at: DateTime<Utc>,  // Invitation expiration
    pub accepted_at: Option<DateTime<Utc>>, // Acceptance timestamp
    pub created_at: DateTime<Utc>,   // Creation timestamp
    pub updated_at: DateTime<Utc>,   // Last update timestamp
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvitationStatus {
    Pending,   // Awaiting user response
    Accepted,  // User joined workspace
    Expired,   // Past expiration date
    Revoked,   // Cancelled by sender/admin
}
```

### Request/Response Models
```rust
// Create invitation
pub struct CreateInvitationRequest {
    pub workspace_id: Uuid,          // Target workspace
    pub invited_email: String,        // Email to invite
    pub role_name: String,           // Role to assign (admin, editor, member, viewer)
    pub expires_in_hours: Option<i64>, // Custom expiration (default: 168 hours)
}

// Accept invitation
pub struct AcceptInvitationRequest {
    pub invitation_token: String,    // Token from invitation URL
}

// Creation response
pub struct CreateInvitationResponse {
    pub invitation: WorkspaceInvitation, // Created invitation
    pub invitation_url: String,           // Acceptance URL
}

// Acceptance response
pub struct AcceptInvitationResponse {
    pub invitation: WorkspaceInvitation,      // Updated invitation
    pub workspace_member: WorkspaceMember,    // Created membership
}
```

## Security & Validation

### Validation Rules
```rust
// Email validation
pub fn validate_email(email: &str) -> Result<(), String>
// Token validation (36-char UUID v7 format)
pub fn validate_invitation_token(token: &str) -> Result<(), String>
// State management
pub fn can_accept(status: &InvitationStatus, expires_at: DateTime<Utc>) -> bool
pub fn can_revoke(status: &InvitationStatus) -> bool
```

### Security Features
- **UUID v7 Tokens**: Time-based sortable unique tokens, single-use
- **Format Validation**: UUID format validation
- **Expiration Handling**: Configurable default duration with maximum limits
- **Case-Insensitive Email**: Stored in lowercase, lookup ignores case
- **Duplicate Prevention**: One pending invitation per email per workspace
- **Access Control**: Requires `INVITE_MEMBERS` permission

### Common Errors
| Context | Error Message |
|---------|---------------|
| Empty email | "Email cannot be empty" |
| Invalid email | "Invalid email format" |
| Invalid token | "Invalid invitation token format" |
| No permission | "You don't have permission to invite members" |
| Already member | "You are already a member of this workspace" |
| Expired | "Invitation has expired" |

## Usage Examples

### Basic Invitation
```rust
let invitation_request = CreateInvitationRequest {
    workspace_id: workspace.id,
    invited_email: "newuser@example.com".to_string(),
    role_name: "member".to_string(),
    expires_in_hours: Some(DEFAULT_DURATION), // Configurable default
};

let response = create_invitation(&mut conn, invitation_request, inviter.id).await?;
println!("Invitation URL: {}", response.invitation_url);
```

### Accept Invitation
```rust
let accept_request = AcceptInvitationRequest {
    invitation_token: "018f1234-5678-9abc-def0-123456789abc".to_string(),
};

let response = accept_invitation(&mut conn, accept_request, user.id).await?;
// Creates WorkspaceMember automatically
println!("Successfully joined workspace!");
```

### Bulk Invitations
```rust
let emails = vec![
    "user1@example.com".to_string(),
    "user2@example.com".to_string(),
    "user3@example.com".to_string(),
];

let responses = bulk_create_invitations(
    &mut conn,
    workspace.id,
    emails,
    "viewer".to_string(),
    inviter.id,
    Some(SHORT_DURATION), // Short-term expiration
).await?;

println!("Sent {} invitations", responses.len());
```

## Database Schema

```sql
CREATE TABLE workspace_invitations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    invited_email TEXT NOT NULL,
    invited_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    invitation_token TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'accepted', 'expired', 'revoked')),
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Core performance indexes
CREATE INDEX idx_workspace_invitations_workspace ON workspace_invitations(workspace_id);
CREATE INDEX idx_workspace_invitations_email ON workspace_invitations(invited_email);
CREATE INDEX idx_workspace_invitations_token ON workspace_invitations(invitation_token);
CREATE INDEX idx_workspace_invitations_status ON workspace_invitations(status);
CREATE INDEX idx_workspace_invitations_expires_at ON workspace_invitations(expires_at);

-- Prevent duplicate pending invitations
CREATE UNIQUE INDEX idx_workspace_invitations_workspace_email_pending
ON workspace_invitations(workspace_id, invited_email)
WHERE status = 'pending';
```

## Configuration

```rust
pub const DEFAULT_INVITATION_EXPIRATION_HOURS: i64; // Configurable default duration
pub const MAX_INVITATION_EXPIRATION_HOURS: i64;      // Maximum allowed duration
pub const MAX_BULK_INVITATIONS: usize;               // Bulk operation limit
```