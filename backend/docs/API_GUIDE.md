# Developer API Guide

Service layer API reference and usage examples for the multi-tenant workspace-based RBAC system.

## Quick API Reference

| Area | Key Functions |
|------|--------------|
| **Users** | `register_user`, `login_user`, `validate_session`, `logout_user`, `get_user_by_id`, `update_password` |
| **Workspaces** | `create_workspace`, `get_workspace`, `list_user_workspaces`, `update_workspace_owner`, `can_access_workspace` |
| **Members** | `create_workspace_member`, `update_workspace_member`, `remove_workspace_member`, `list_workspace_members` |
| **Permissions** | `validate_workspace_permission`, `require_workspace_permission`, `get_user_workspace_permissions` |
| **Roles** | `create_default_roles`, `get_role_by_name`, `list_workspace_roles`, `get_role` |
| **Invitations** | `create_invitation`, `accept_invitation`, `revoke_invitation`, `bulk_create_invitations`, `get_invitation_by_token` |
| **Sessions** | `cleanup_expired_sessions`, `revoke_all_user_sessions`, `revoke_session_by_token`, `user_has_active_sessions` |
| **Validation** | `validate_email`, `validate_password`, `validate_workspace_name`, `validate_session_token` |

---

## Core Service APIs

### User Authentication
```rust
// User registration (8+ char password, email validation)
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User>

// Combined user + workspace creation
pub async fn register_user_with_workspace(
    conn: &mut DbConn,
    request: UserWorkspaceRegistrationRequest
) -> Result<UserWorkspaceResult>

// Authentication with dual-token generation (JWT + session)
pub async fn login_user(conn: &mut DbConn, login_user: LoginUser) -> Result<LoginResult>

// Session validation and management
pub async fn validate_session(conn: &mut DbConn, session_token: &str) -> Result<User>
pub async fn logout_user(conn: &mut DbConn, session_token: &str) -> Result<()>
pub async fn refresh_session(conn: &mut DbConn, session_token: &str, hours_to_extend: i64) -> Result<String>

// JWT access token refresh
pub async fn refresh_access_token(conn: &mut DbConn, refresh_token: &str) -> Result<RefreshTokenResult>
```

### Workspace Management
```rust
// Workspace creation with automatic setup (creates default roles + owner as admin)
pub async fn create_workspace(
    conn: &mut DbConn,
    request: CreateWorkspaceRequest
) -> Result<CompleteWorkspaceResult>

// Workspace creation with initial team
pub async fn create_workspace_with_members(
    conn: &mut DbConn,
    request: CreateWorkspaceWithMembersRequest
) -> Result<CompleteWorkspaceResult>

// Basic operations
pub async fn get_workspace(conn: &mut DbConn, id: Uuid) -> Result<Workspace>
pub async fn list_user_workspaces(conn: &mut DbConn, owner_id: Uuid) -> Result<Vec<Workspace>>
pub async fn list_workspaces(conn: &mut DbConn) -> Result<Vec<Workspace>>
pub async fn delete_workspace(conn: &mut DbConn, id: Uuid) -> Result<u64>

// Critical ownership and access functions
pub async fn update_workspace_owner(
    conn: &mut DbConn,
    workspace_id: Uuid,
    current_owner_id: Uuid,
    new_owner_id: Uuid,
) -> Result<Workspace>

pub async fn can_access_workspace(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool>

pub async fn validate_workspace_ownership(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<()>
```

### Member Management
```rust
// Member creation and assignment
pub async fn create_workspace_member(
    conn: &mut DbConn,
    new_member: NewWorkspaceMember,
) -> Result<WorkspaceMember>

// Member updates and role changes
pub async fn update_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    update_member: UpdateWorkspaceMember,
) -> Result<WorkspaceMember>

pub async fn remove_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<u64>

// Member queries and lookups
pub async fn list_workspace_members(
    conn: &mut DbConn,
    workspace_id: Uuid,
) -> Result<Vec<WorkspaceMember>>

pub async fn list_user_workspaces(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<Vec<WorkspaceMember>>

pub async fn get_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<WorkspaceMember>

pub async fn get_workspace_member_optional(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<Option<WorkspaceMember>>

pub async fn is_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool>
```

### Role Management
```rust
// Create default 4-tier role system for workspace
pub async fn create_default_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>

// Role lookup and listing
pub async fn get_role_by_name(
    conn: &mut DbConn,
    workspace_id: Uuid,
    role_name: &str
) -> Result<Role>

pub async fn list_workspace_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>

// Individual role lookup
pub async fn get_role(conn: &mut DbConn, id: Uuid) -> Result<Role>
```

### Permission Validation
```rust
// Permission checking functions
pub async fn validate_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    required_permission: &str,
) -> Result<bool>

pub async fn require_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    required_permission: &str,
) -> Result<()>

pub async fn validate_any_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    required_permissions: &[&str],
) -> Result<bool>

pub async fn validate_all_workspace_permissions(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    required_permissions: &[&str],
) -> Result<bool>

// User permission lookup
pub async fn get_user_workspace_permissions(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<String>>
```

### Invitations
```rust
// Invitation creation with role assignment
pub async fn create_invitation(
    conn: &mut DbConn,
    request: CreateInvitationRequest,
    inviter_id: Uuid,
) -> Result<CreateInvitationResponse>

// Acceptance and management
pub async fn accept_invitation(
    conn: &mut DbConn,
    request: AcceptInvitationRequest,
    user_id: Uuid,
) -> Result<AcceptInvitationResponse>

pub async fn revoke_invitation(
    conn: &mut DbConn,
    request: RevokeInvitationRequest,
    revoker_id: Uuid,
) -> Result<WorkspaceInvitation>

// Bulk operations
pub async fn bulk_create_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
    emails: Vec<String>,
    role_name: String,
    inviter_id: Uuid,
    expires_in_hours: Option<i64>,
) -> Result<Vec<CreateInvitationResponse>>

// Additional invitation utilities
pub async fn get_invitation_by_token(
    conn: &mut DbConn,
    token: &str,
) -> Result<WorkspaceInvitation>

pub async fn list_workspace_invitations(
    conn: &mut DbConn,
    workspace_id: Uuid,
) -> Result<Vec<WorkspaceInvitation>>

pub async fn list_user_sent_invitations(
    conn: &mut DbConn,
    user_id: Uuid,
) -> Result<Vec<WorkspaceInvitation>>

pub async fn cleanup_expired_invitations(conn: &mut DbConn) -> Result<u64>
```

### Enhanced User Management
```rust
// User lookup and utilities
pub async fn get_user_by_id(conn: &mut DbConn, user_id: Uuid) -> Result<Option<User>>
pub async fn is_email_available(conn: &mut DbConn, email: &str) -> Result<bool>
pub async fn update_password(conn: &mut DbConn, user_id: Uuid, new_password: &str) -> Result<()>
pub async fn get_session_info(conn: &mut DbConn, session_token: &str) -> Result<Option<UserSession>>

// User workspace memberships
pub async fn get_user_active_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<Vec<UserSession>>
pub async fn revoke_all_user_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<u64>
```

### Session Management
```rust
// Session cleanup and maintenance
pub async fn cleanup_expired_sessions(conn: &mut DbConn) -> Result<u64>
pub async fn revoke_session_by_token(conn: &mut DbConn, session_token: &str) -> Result<()>
pub async fn user_has_active_sessions(conn: &mut DbConn, user_id: Uuid) -> Result<bool>
pub async fn extend_all_user_sessions(conn: &mut DbConn, user_id: Uuid, hours_to_extend: i64) -> Result<u64>
```

### Input Validation Utilities
```rust
// Core validation functions (from validation.rs)
pub fn validate_email(email: &str) -> Result<()>
pub fn validate_password(password: &str) -> Result<()>
pub fn validate_workspace_name(name: &str) -> Result<()>
pub fn validate_full_name(full_name: &Option<String>) -> Result<()>
pub fn validate_session_token(token: &str) -> Result<()>
pub fn validate_uuid(uuid_str: &str) -> Result<Uuid>
pub fn validate_required_string(input: &str, field_name: &str) -> Result<String>
pub fn sanitize_string(input: &str) -> String
```

### Key Request Models
```rust
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub owner_id: Uuid,
}

pub struct WorkspaceMemberRequest {
    pub user_id: Uuid,
    pub role_name: String,  // "admin", "editor", "member", "viewer"
}

pub struct CreateInvitationRequest {
    pub workspace_id: Uuid,
    pub invited_email: String,
    pub role_name: String,
    pub expires_in_hours: Option<i64>,
}
```

## Essential Usage Examples

### User Authentication
```rust
// Register + Login
let user = register_user(&mut conn, RegisterUser {
    email: "user@example.com".to_string(),
    password: "securepassword123".to_string(),
    confirm_password: "securepassword123".to_string(),
    full_name: Some("John Doe".to_string()),
}).await?;

let login_result = login_user(&mut conn, LoginUser {
    email: "user@example.com".to_string(),
    password: "securepassword123".to_string(),
}).await?;

// Returns both JWT access token (15 min) and refresh token (30 days)
// - Use login_result.access_token in API Authorization header
// - Use login_result.refresh_token to get new access tokens

// When access token expires, refresh it
let new_token = refresh_access_token(&mut conn, &login_result.refresh_token).await?;

// Validate session (uses refresh token)
let user = validate_session(&mut conn, &login_result.refresh_token).await?;

// Logout (invalidates refresh token)
logout_user(&mut conn, &login_result.refresh_token).await?;
```

### Workspace Setup
```rust
// Create workspace with automatic role setup
let workspace_result = create_workspace(&mut conn, CreateWorkspaceRequest {
    name: "Team Workspace".to_string(),
    owner_id: user.id,
}).await?;
// Creates: workspace + 4 default roles + owner as admin

// Create workspace with initial team
let workspace_result = create_workspace_with_members(&mut conn, CreateWorkspaceWithMembersRequest {
    name: "Project Workspace".to_string(),
    owner_id: user.id,
    members: vec![
        WorkspaceMemberRequest {
            user_id: editor_user.id,
            role_name: "editor".to_string(),
        },
        WorkspaceMemberRequest {
            user_id: member_user.id,
            role_name: "member".to_string(),
        },
    ],
}).await?;
```

### Member Management
```rust
// Add member with role
let member = add_workspace_member(&mut conn, workspace.id, user.id, editor_role.id).await?;

// Update member role
let updated = update_workspace_member_role(&mut conn, workspace.id, user.id, admin_role.id).await?;

// List members
let members = list_workspace_members(&mut conn, workspace.id).await?;
```

### Invitation Workflow
```rust
// Create invitation
let invitation_result = create_invitation(&mut conn, CreateInvitationRequest {
    workspace_id: workspace.id,
    invited_email: "teammate@example.com".to_string(),
    role_name: "member".to_string(),
    expires_in_hours: Some(168), // 7 days (note: this is for invitations, not sessions)
}, inviter.id).await?;

// Accept invitation (creates membership automatically)
let accept_result = accept_invitation(&mut conn, AcceptInvitationRequest {
    invitation_token: invitation_result.invitation.invitation_token,
}, new_user.id).await?;
```

### Bulk Operations
```rust
// Bulk invite team members
let emails = vec![
    "member1@example.com".to_string(),
    "member2@example.com".to_string(),
    "member3@example.com".to_string(),
];

let invitation_results = bulk_create_invitations(
    &mut conn,
    workspace.id,
    emails,
    "member".to_string(),
    inviter.id,
    Some(168), // 7 days (note: this is for invitations, not sessions)
).await?;

// Cleanup expired sessions/invitations
let cleaned_sessions = cleanup_expired_sessions(&mut conn).await?;
let cleaned_invitations = cleanup_expired_invitations(&mut conn).await?;
```

## Development Best Practices

### Use Type-Safe Role Constants
```rust
// ✅ Preferred: Use centralized constants
use backend::models::roles::{ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE};

let member_request = WorkspaceMemberRequest {
    user_id: user.id,
    role_name: ADMIN_ROLE.to_string(),
};
```

### Use Comprehensive Creation Methods
```rust
// ✅ Preferred: Automatic workspace setup
let result = create_workspace(&mut conn, workspace_request).await?;
// Creates: workspace + default roles + owner as admin

// ❌ Avoid: Manual multi-step creation
```

## Error Handling Guide

### Error Types

The system uses a comprehensive error hierarchy with specific error types for different scenarios:

```rust
#[derive(Debug, Error)]
pub enum Error {
    Sqlx(#[from] sqlx::Error),           // Database errors
    Validation(String),                   // Input validation errors
    NotFound(String),                      // Resource not found
    Forbidden(String),                    // Permission denied
    Conflict(String),                      // Resource conflicts
    Authentication(String),               // Invalid credentials
    InvalidToken(String),                 // Invalid/expired session tokens
    SessionExpired(String),               // Session expiration errors
    Internal(String),                      // System errors
}
```

### Error Handling Patterns

#### 1. Comprehensive Error Handling
```rust
match service_function(&mut conn, request).await {
    Ok(result) => handle_success(result),
    Err(Error::Validation(msg)) => {
        log::warn!("Validation error: {}", msg);
        return Err(create_api_error(400, msg));
    },
    Err(Error::Authentication(msg)) => {
        log::info!("Authentication failed: {}", msg);
        return Err(create_api_error(401, "Invalid credentials"));
    },
    Err(Error::InvalidToken(msg) | Error::SessionExpired(msg)) => {
        log::info!("Session error: {}", msg);
        return Err(create_api_error(401, "Session expired"));
    },
    Err(Error::Forbidden(msg)) => {
        log::warn!("Access forbidden: {}", msg);
        return Err(create_api_error(403, "Access denied"));
    },
    Err(Error::NotFound(msg)) => {
        log::info!("Resource not found: {}", msg);
        return Err(create_api_error(404, msg));
    },
    Err(Error::Conflict(msg)) => {
        log::warn!("Conflict error: {}", msg);
        return Err(create_api_error(409, msg));
    },
    Err(Error::Sqlx(db_error)) => {
        log::error!("Database error: {}", db_error);
        return Err(create_api_error(500, "Database error"));
    },
    Err(Error::Internal(msg)) => {
        log::error!("Internal error: {}", msg);
        return Err(create_api_error(500, "Internal server error"));
    }
}
```

#### 2. User Registration Error Handling
```rust
match register_user(&mut conn, register_request).await {
    Ok(user) => create_user_session(user),
    Err(Error::Validation(msg)) => {
        match msg.as_str() {
            "Password must meet minimum length requirements" =>
                show_field_error("password", "Password too short"),
            "Passwords do not match" =>
                show_field_error("confirm_password", "Passwords don't match"),
            "Email cannot be empty" =>
                show_field_error("email", "Email is required"),
            _ => show_general_error("Validation failed")
        }
    },
    Err(Error::Conflict(msg)) if msg.contains("duplicate key value violates unique constraint") => {
        show_field_error("email", "Email already registered");
    },
    Err(error) => {
        log::error!("Registration error: {}", error);
        show_general_error("Registration failed. Please try again.");
    }
}
```

#### 3. Authentication Error Handling
```rust
match login_user(&mut conn, login_request).await {
    Ok(login_result) => {
        create_user_session(login_result);
        redirect_to_dashboard();
    },
    Err(Error::Authentication(_)) => {
        show_error("Invalid email or password");
        increment_login_attempts();
    },
    Err(Error::Validation(msg)) => {
        show_error(&format!("Please fill in all fields: {}", msg));
    },
    Err(error) => {
        log::error!("Login error: {}", error);
        show_error("Login failed. Please try again.");
    }
}
```

#### 4. Workspace Access Error Handling
```rust
match can_access_workspace(&mut conn, workspace_id, user.id).await {
    Ok(true) => {
        // User has access - proceed
        handle_workspace_request();
    },
    Ok(false) => {
        log::warn!("User {} attempted to access workspace {}", user.id, workspace_id);
        return Err(create_api_error(403, "You don't have access to this workspace"));
    },
    Err(Error::NotFound(_)) => {
        return Err(create_api_error(404, "Workspace not found"));
    },
    Err(Error::InvalidToken(_) | Error::SessionExpired(_)) => {
        return Err(create_api_error(401, "Please log in to continue"));
    },
    Err(error) => {
        log::error!("Workspace access check failed: {}", error);
        return Err(create_api_error(500, "Access check failed"));
    }
}
```

#### 5. Session Management Error Handling
```rust
match validate_session(&mut conn, session_token).await {
    Ok(user) => {
        // Valid session - proceed with request
        handle_authenticated_request(user);
    },
    Err(Error::InvalidToken(_) | Error::SessionExpired(_)) => {
        // Clear invalid session cookie
        clear_session_cookie();
        redirect_to_login_with_message("Your session has expired. Please log in again.");
    },
    Err(error) => {
        log::error!("Session validation error: {}", error);
        clear_session_cookie();
        redirect_to_login_with_message("Authentication error. Please log in again.");
    }
}
```

### Error Response Format

#### API Error Response Structure
```rust
pub struct ApiError {
    pub error: String,
    pub message: String,
    pub details: Option<String>,
    pub code: Option<String>,
}

// Example error responses
{
    "error": "validation_error",
    "message": "Email is required",
    "details": "The email field cannot be empty",
    "code": "EMAIL_REQUIRED"
}

{
    "error": "authentication_error",
    "message": "Invalid credentials",
    "details": null,
    "code": "INVALID_CREDENTIALS"
}
```

### Common Error Scenarios

| Scenario | Error Type | HTTP Status | User Message |
|----------|------------|-------------|--------------|
| Invalid email format | `Validation` | 400 | "Invalid email format" |
| Password too short | `Validation` | 400 | "Password must meet minimum length requirements" |
| Email already exists | `Conflict` | 409 | "Email already registered" |
| Invalid login credentials | `Authentication` | 401 | "Invalid email or password" |
| Session expired | `SessionExpired` | 401 | "Session expired. Please log in again" |
| No workspace access | `Forbidden` | 403 | "You don't have access to this workspace" |
| Workspace not found | `NotFound` | 404 | "Workspace not found" |
| Database connection failed | `Sqlx` | 500 | "Database error. Please try again" |
| Internal system error | `Internal` | 500 | "Internal server error" |

## Key Architecture

- **Three-Layer**: Service → Query → Model architecture
- **RBAC System**: 4-tier roles (Admin > Editor > Member > Viewer)
- **Comprehensive Permissions**: Fine-grained permissions across workspace, content, and member categories
- **Multi-Tenant**: Complete workspace isolation with shared users
- **Session-Based**: UUID v7 tokens with Argon2 password hashing

## Environment Setup

```bash
# Required environment variables
BUILDSCALE__DATABASE__USER=your_db_user
BUILDSCALE__DATABASE__PASSWORD=your_db_password
BUILDSCALE__DATABASE__HOST=localhost
BUILDSCALE__DATABASE__PORT=5432
BUILDSCALE__DATABASE__DATABASE=your_db_name

# Development commands
cargo build                    # Build project
cargo test                      # Run all tests
sqlx migrate run               # Run migrations
```