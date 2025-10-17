# User, Workspace & Member Management

Global user accounts, workspace creation, and member management with role-based access control.

## Core APIs

### User Management
```rust
// User registration (8+ char password, email validation)
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User>

// Combined user + workspace creation
pub async fn register_user_with_workspace(
    conn: &mut DbConn,
    request: UserWorkspaceRegistrationRequest
) -> Result<UserWorkspaceResult>

// User authentication and session creation
pub async fn login_user(conn: &mut DbConn, login_user: LoginUser) -> Result<LoginResult>

// User utilities
pub async fn get_user_by_id(conn: &mut DbConn, user_id: Uuid) -> Result<Option<User>>
pub async fn update_password(conn: &mut DbConn, user_id: Uuid, new_password: &str) -> Result<()>
pub async fn is_email_available(conn: &mut DbConn, email: &str) -> Result<bool>
```

### Workspace Management
```rust
// Workspace creation with automatic setup (creates default roles + owner as admin)
pub async fn create_workspace(
    conn: &mut DbConn,
    request: CreateWorkspaceRequest
) -> Result<CompleteWorkspaceResult>

// Workspace creation with initial team members
pub async fn create_workspace_with_members(
    conn: &mut DbConn,
    request: CreateWorkspaceWithMembersRequest
) -> Result<CompleteWorkspaceResult>

// Ownership transfer (ensures new owner gets admin role)
pub async fn update_workspace_owner(
    conn: &mut DbConn,
    workspace_id: Uuid,
    current_owner_id: Uuid,
    new_owner_id: Uuid,
) -> Result<Workspace>

// Basic operations
pub async fn get_workspace(conn: &mut DbConn, id: Uuid) -> Result<Workspace>
pub async fn list_user_workspaces(conn: &mut DbConn, owner_id: Uuid) -> Result<Vec<Workspace>>
pub async fn delete_workspace(conn: &mut DbConn, id: Uuid) -> Result<u64>
```

### Member Management
```rust
// Member assignment and role updates
pub async fn add_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    role_id: Uuid,
) -> Result<WorkspaceMember>

pub async fn update_workspace_member_role(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    new_role_id: Uuid,
) -> Result<WorkspaceMember>

pub async fn remove_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<u64>

// Member queries
pub async fn list_workspace_members(
    conn: &mut DbConn,
    workspace_id: Uuid,
) -> Result<Vec<WorkspaceMember>>

pub async fn is_workspace_member(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool>
```

## Data Models

### Core Entities
```rust
pub struct User {
    pub id: Uuid,                    // Primary key (UUID v7)
    pub email: String,               // Globally unique email
    pub password_hash: String,       // Argon2 hashed password
    pub full_name: Option<String>,   // Optional display name
    pub created_at: DateTime<Utc>,   // Account creation time
    pub updated_at: DateTime<Utc>,   // Last update time
}

pub struct Workspace {
    pub id: Uuid,                    // Primary key (UUID v7)
    pub name: String,                 // Workspace name (1-100 chars)
    pub owner_id: Uuid,               // Owner user ID (RESTRICT constraint)
    pub created_at: DateTime<Utc>,   // Creation time
    pub updated_at: DateTime<Utc>,   // Last update time
}

pub struct WorkspaceMember {
    pub workspace_id: Uuid,      // Workspace ID (part of composite PK)
    pub user_id: Uuid,          // User ID (part of composite PK)
    pub role_id: Uuid,          // Assigned role ID
    pub created_at: DateTime<Utc>, // Membership creation time
    pub updated_at: DateTime<Utc>, // Last update time
}
```

## Key Features

- **Global User Accounts**: Single email/password works across all workspaces
- **Single-Owner Model**: Each workspace has exactly one owner with full control
- **Role-Based Access**: Four-tier role system (Admin > Editor > Member > Viewer)
- **Multi-Device Sessions**: Users can maintain concurrent sessions
- **Workspace Isolation**: Complete data separation between workspaces

## Usage Examples

### User Registration + Workspace
```rust
let request = UserWorkspaceRegistrationRequest {
    email: "newuser@example.com".to_string(),
    password: "SecurePassword123!".to_string(),
    confirm_password: "SecurePassword123!".to_string(),
    full_name: Some("Jane Smith".to_string()),
    workspace_name: "Jane's Workspace".to_string(),
};

let result = register_user_with_workspace(&mut conn, request).await?;
// Creates user + workspace + 4 default roles + owner as admin
```

### Team Workspace Setup
```rust
let workspace_request = CreateWorkspaceWithMembersRequest {
    name: "Team Project".to_string(),
    owner_id: owner.id,
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
};

let workspace_result = create_workspace_with_members(&mut conn, workspace_request).await?;
```

### Member Management
```rust
// Add member with role
let member = add_workspace_member(&mut conn, workspace.id, user.id, editor_role.id).await?;

// Update member role
let updated = update_workspace_member_role(&mut conn, workspace.id, user.id, admin_role.id).await?;

// Remove member
let removed = remove_workspace_member(&mut conn, workspace.id, user.id).await?;
```

## Database Schema

### Core Tables
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    full_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE workspace_members (
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workspace_id, user_id)
);
```