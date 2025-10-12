# Users, Roles, and Workspaces System Documentation

This comprehensive documentation covers the complete user-role-workspace management system implemented in the backend. The system provides a multi-tenant architecture with role-based access control (RBAC), workspace isolation, and comprehensive user management.

## Table of Contents

1. [System Architecture](#system-architecture)
2. [User Management System](#user-management-system)
3. [Role Management System](#role-management-system)
4. [Workspace Management System](#workspace-management-system)
5. [Workspace Members System](#workspace-members-system)
6. [Service Layer API](#service-layer-api)
7. [Request/Response Models](#requestresponse-models)
8. [Validation and Error Handling](#validation-and-error-handling)
9. [Database Schema and Relationships](#database-schema-and-relationships)
10. [Usage Examples and Patterns](#usage-examples-and-patterns)
11. [Security Considerations](#security-considerations)
12. [Best Practices and Guidelines](#best-practices-and-guidelines)

## System Architecture

### Overview

The system implements a **multi-tenant workspace-based architecture** with the following key characteristics:

- **Workspace Isolation**: Each workspace is completely isolated from others
- **Role-Based Access Control (RBAC)**: Three-tier role system (Admin > Editor > Viewer)
- **Single Owner Model**: Each workspace has exactly one owner with full control
- **Flexible Membership**: Users can be members of multiple workspaces with different roles
- **Centralized Role Management**: Default roles are automatically created for each workspace

### Entity Relationships

```
Users (1) ←→ (N) Workspaces
   ↓                   ↓
   └── Workspace Members ──→ Roles (per workspace)
```

- **Users** can be members of multiple workspaces
- **Workspaces** have exactly one owner (a user)
- **Roles** are scoped to individual workspaces
- **Workspace Members** represent the many-to-many relationship between users and workspaces with specific roles

### Data Flow

1. **User Registration** → Creates user account
2. **Workspace Creation** → Creates workspace with default roles
3. **Member Assignment** → Users assigned to workspaces with specific roles
4. **Access Control** → Role-based permissions determine workspace access

## User Management System

### User Models

#### `User` - Complete User Entity
```rust
pub struct User {
    pub id: Uuid,                    // Primary key
    pub email: String,               // Unique email address
    pub password_hash: String,       // Argon2 hashed password
    pub full_name: Option<String>,   // Optional display name
    pub created_at: DateTime<Utc>,   // Creation timestamp
    pub updated_at: DateTime<Utc>,   // Last update timestamp
}
```

#### `RegisterUser` - User Registration Request
```rust
pub struct RegisterUser {
    pub email: String,              // User email (must be unique)
    pub password: String,           // Plain text password (will be hashed)
    pub confirm_password: String,    // Password confirmation
    pub full_name: Option<String>,  // Optional display name
}
```

#### `NewUser` - Internal User Creation
```rust
pub struct NewUser {
    pub email: String,              // User email
    pub password_hash: String,      // Pre-hashed password
    pub full_name: Option<String>,  // Optional display name
}
```

#### `UpdateUser` - User Update Request
```rust
pub struct UpdateUser {
    pub email: Option<String>,         // New email (optional)
    pub password_hash: Option<String>, // New password hash (optional)
    pub full_name: Option<String>,     // New display name (optional)
}
```

### User Services

#### User Registration
```rust
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User>
```

**Features:**
- Password validation (minimum 8 characters)
- Password confirmation matching
- Argon2 password hashing
- Email uniqueness enforcement via database constraints

#### User Registration with Workspace
```rust
pub async fn register_user_with_workspace(
    conn: &mut DbConn,
    request: UserWorkspaceRegistrationRequest
) -> Result<UserWorkspaceResult>
```

**Features:**
- Creates user and first workspace in single transaction
- Automatic default role creation
- Owner automatically assigned admin role
- Workspace name validation (1-100 characters)

#### Password Management
```rust
pub fn generate_password_hash(password: &str) -> Result<String>
pub fn verify_password(password: &str, hash: &str) -> Result<bool>
```

**Security Features:**
- **Argon2 Algorithm**: Industry-standard password hashing
- **Salt Generation**: Unique salt per password using `OsRng`
- **Secure Verification**: Constant-time comparison to prevent timing attacks

### User Validation Rules

| Field | Validation | Error Message |
|-------|------------|---------------|
| `email` | Must be unique (database constraint) | "duplicate key value violates unique constraint" |
| `password` | Minimum 8 characters | "Password must be at least 8 characters long" |
| `password` | Must match `confirm_password` | "Passwords do not match" |
| `email` | Valid email format (application-level) | Varies by implementation |

## Role Management System

### Role Constants and Enum

#### Role Constants
```rust
pub const ADMIN_ROLE: &str = "admin";    // Full administrative access
pub const EDITOR_ROLE: &str = "editor";  // Can create and edit content
pub const VIEWER_ROLE: &str = "viewer";  // Read-only access
pub const DEFAULT_ROLES: [&str; 3] = [ADMIN_ROLE, EDITOR_ROLE, VIEWER_ROLE];
```

#### `WorkspaceRole` Enum - Type Safety
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceRole {
    Admin,   // Full administrative access
    Editor,  // Can create and edit content
    Viewer,  // Read-only access
}
```

**Enum Methods:**
```rust
impl WorkspaceRole {
    pub fn as_str(&self) -> &'static str     // Get string representation
    pub fn name(&self) -> String            // Get name as String
    pub fn from_str(role: &str) -> Option<Self> // Create from string
}
```

### Role Models

#### `Role` - Complete Role Entity
```rust
pub struct Role {
    pub id: Uuid,                    // Primary key
    pub workspace_id: Uuid,          // Associated workspace
    pub name: String,                // Role name (admin, editor, viewer, custom)
    pub description: Option<String>, // Optional description
}
```

#### `NewRole` - Role Creation
```rust
pub struct NewRole {
    pub workspace_id: Uuid,          // Target workspace
    pub name: String,                // Role name
    pub description: Option<String>, // Optional description
}
```

#### `UpdateRole` - Role Update
```rust
pub struct UpdateRole {
    pub name: Option<String>,        // New name (optional)
    pub description: Option<String>, // New description (optional)
}
```

### Role Descriptions

Built-in descriptions for default roles:

```rust
pub mod descriptions {
    pub const ADMIN: &str = "Full administrative access to workspace";
    pub const EDITOR: &str = "Can create and edit content";
    pub const VIEWER: &str = "Read-only access to workspace";

    pub fn for_role(role_name: &str) -> &'static str {
        match role_name {
            ADMIN_ROLE => ADMIN,
            EDITOR_ROLE => EDITOR,
            VIEWER_ROLE => VIEWER,
            _ => "Custom role",
        }
    }
}
```

### Role Services

#### Default Role Creation
```rust
pub async fn create_default_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>
```

**Features:**
- Automatically creates admin, editor, and viewer roles
- Assigns appropriate descriptions
- Returns all created roles for immediate use

#### Role Management
```rust
pub async fn create_single_role(conn: &mut DbConn, new_role: NewRole) -> Result<Role>
pub async fn get_role(conn: &mut DbConn, id: Uuid) -> Result<Role>
pub async fn list_workspace_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>
pub async fn get_role_by_name(conn: &mut DbConn, workspace_id: Uuid, role_name: &str) -> Result<Role>
```

### Role Hierarchy and Permissions

| Role | Permissions | Typical Use Cases |
|------|-------------|------------------|
| **Admin** | Full workspace control, user management, role assignment, workspace settings | Workspace owners, administrators |
| **Editor** | Create and edit content, invite viewers, moderate content | Content creators, team leads |
| **Viewer** | Read-only access to content and discussions | Clients, stakeholders, read-only team members |

### Role Validation Rules

| Field | Validation | Error Message |
|-------|------------|---------------|
| `name` | Cannot be empty | "Role name cannot be empty" |
| `name` | Maximum 100 characters | "Role name must be less than 100 characters" |
| `name` | Must be unique per workspace | "Role '{name}' already exists in this workspace" |
| `description` | Maximum 500 characters | "Role description must be less than 500 characters" |

## Workspace Management System

### Workspace Models

#### `Workspace` - Complete Workspace Entity
```rust
pub struct Workspace {
    pub id: Uuid,                    // Primary key
    pub name: String,                // Workspace name (1-100 chars)
    pub owner_id: Uuid,              // Workspace owner (user ID)
    pub created_at: DateTime<Utc>,   // Creation timestamp
    pub updated_at: DateTime<Utc>,   // Last update timestamp
}
```

#### `NewWorkspace` - Workspace Creation
```rust
pub struct NewWorkspace {
    pub name: String,        // Workspace name
    pub owner_id: Uuid,      // Owner user ID
}
```

#### `UpdateWorkspace` - Workspace Update
```rust
pub struct UpdateWorkspace {
    pub name: Option<String>,     // New name (optional)
    pub owner_id: Option<Uuid>,   // New owner (optional)
}
```

### Workspace Services

#### Simplified Workspace Creation
```rust
pub async fn create_workspace(
    conn: &mut DbConn,
    request: CreateWorkspaceRequest
) -> Result<CompleteWorkspaceResult>
```

**Features:**
- Creates workspace with default roles (admin, editor, viewer)
- Automatically adds owner as admin member
- Returns complete workspace setup with roles and members
- Validates workspace name (1-100 characters, not empty)

#### Workspace Creation with Members
```rust
pub async fn create_workspace_with_members(
    conn: &mut DbConn,
    request: CreateWorkspaceWithMembersRequest
) -> Result<CompleteWorkspaceResult>
```

**Features:**
- All features of basic creation
- Adds multiple initial members with specified roles
- Automatically deduplicates owner if listed in members
- Validates all user IDs and role names

#### Ownership Transfer
```rust
pub async fn update_workspace_owner(
    conn: &mut DbConn,
    workspace_id: Uuid,
    current_owner_id: Uuid,
    new_owner_id: Uuid,
) -> Result<Workspace>
```

**Features:**
- Validates current ownership
- Prevents self-transfer
- Automatically adds new owner as admin member
- Updates previous owner to retain admin access

#### Workspace Management
```rust
pub async fn get_workspace(conn: &mut DbConn, id: Uuid) -> Result<Workspace>
pub async fn list_user_workspaces(conn: &mut DbConn, owner_id: Uuid) -> Result<Vec<Workspace>>
pub async fn list_workspaces(conn: &mut DbConn) -> Result<Vec<Workspace>>
pub async fn delete_workspace(conn: &mut DbConn, id: Uuid) -> Result<u64>
```

#### Access Control
```rust
pub async fn validate_workspace_ownership(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool>

pub async fn can_access_workspace(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool>
```

### Workspace Validation Rules

| Field | Validation | Error Message |
|-------|------------|---------------|
| `name` | Cannot be empty or whitespace-only | "Workspace name cannot be empty" |
| `name` | Maximum 100 characters | "Workspace name must be less than 100 characters" |
| `owner_id` | Must reference existing user | Foreign key constraint |
| `owner_id` | Cannot transfer to self | "Cannot transfer ownership to yourself" |

## Workspace Members System

### Member Models

#### `WorkspaceMember` - Complete Member Entity
```rust
pub struct WorkspaceMember {
    pub workspace_id: Uuid,    // Associated workspace
    pub user_id: Uuid,         // Member user ID
    pub role_id: Uuid,         // Assigned role ID
}
```

#### `NewWorkspaceMember` - Member Addition
```rust
pub struct NewWorkspaceMember {
    pub workspace_id: Uuid,    // Target workspace
    pub user_id: Uuid,         // User to add
    pub role_id: Uuid,         // Role to assign
}
```

#### `UpdateWorkspaceMember` - Member Role Update
```rust
pub struct UpdateWorkspaceMember {
    pub role_id: Option<Uuid>, // New role (optional)
}
```

### Member Management Features

#### Automatic Owner Assignment
- Workspace owners are automatically assigned the **admin role**
- Owners cannot be removed from their own workspace
- Ownership transfer preserves previous owner's admin access

#### Role Assignment
- Members can be assigned any valid workspace role
- Role names are validated against existing roles
- Custom roles are supported in addition to default roles

#### Member Uniqueness
- Each user can only have one membership per workspace
- Database constraints prevent duplicate memberships
- Service layer gracefully handles duplicate owner scenarios

### Member Services

#### Member Creation and Management
```rust
// Implemented through workspace services
pub async fn create_workspace_with_members(...) // Creates with initial members
pub async fn update_workspace_owner(...) // Transfers ownership with role management
```

#### Member Validation
- User must exist in the system
- Role must exist in the workspace
- Cannot create duplicate memberships
- Owner always has admin role

## Service Layer API

### User Services

#### Core User Operations
```rust
// User registration with validation and password hashing
pub async fn register_user(conn: &mut DbConn, register_user: RegisterUser) -> Result<User>

// Combined user + workspace registration
pub async fn register_user_with_workspace(
    conn: &mut DbConn,
    request: UserWorkspaceRegistrationRequest
) -> Result<UserWorkspaceResult>

// Password security operations
pub fn generate_password_hash(password: &str) -> Result<String>
pub fn verify_password(password: &str, hash: &str) -> Result<bool>
```

### Workspace Services

#### Workspace Creation and Management
```rust
// Create workspace with automatic setup
pub async fn create_workspace(
    conn: &mut DbConn,
    request: CreateWorkspaceRequest
) -> Result<CompleteWorkspaceResult>

// Create workspace with initial members
pub async fn create_workspace_with_members(
    conn: &mut DbConn,
    request: CreateWorkspaceWithMembersRequest
) -> Result<CompleteWorkspaceResult>

// Ownership transfer with role management
pub async fn update_workspace_owner(
    conn: &mut DbConn,
    workspace_id: Uuid,
    current_owner_id: Uuid,
    new_owner_id: Uuid,
) -> Result<Workspace>

// Basic operations
pub async fn get_workspace(conn: &mut DbConn, id: Uuid) -> Result<Workspace>
pub async fn list_user_workspaces(conn: &mut DbConn, owner_id: Uuid) -> Result<Vec<Workspace>>
pub async fn list_workspaces(conn: &mut DbConn) -> Result<Vec<Workspace>>
pub async fn delete_workspace(conn: &mut DbConn, id: Uuid) -> Result<u64>
```

#### Access Control
```rust
// Validate workspace ownership
pub async fn validate_workspace_ownership(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool>

// Check workspace access (owner or member)
pub async fn can_access_workspace(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<bool>
```

### Role Services

#### Role Management
```rust
// Create default roles for workspace
pub async fn create_default_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>

// Create custom role
pub async fn create_single_role(conn: &mut DbConn, new_role: NewRole) -> Result<Role>

// Role lookup
pub async fn get_role(conn: &mut DbConn, id: Uuid) -> Result<Role>
pub async fn get_role_by_name(
    conn: &mut DbConn,
    workspace_id: Uuid,
    role_name: &str
) -> Result<Role>

// List roles
pub async fn list_workspace_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>
```

## Request/Response Models

### Registration Models

#### `UserWorkspaceRegistrationRequest` - Complete Onboarding
```rust
pub struct UserWorkspaceRegistrationRequest {
    pub email: String,              // User email
    pub password: String,           // User password
    pub confirm_password: String,    // Password confirmation
    pub full_name: Option<String>,  // Optional display name
    pub workspace_name: String,     // Initial workspace name
}
```

### Workspace Creation Models

#### `CreateWorkspaceRequest` - Basic Workspace Creation
```rust
pub struct CreateWorkspaceRequest {
    pub name: String,        // Workspace name (1-100 chars)
    pub owner_id: Uuid,      // Owner user ID
}
```

#### `CreateWorkspaceWithMembersRequest` - Workspace with Initial Team
```rust
pub struct CreateWorkspaceWithMembersRequest {
    pub name: String,                              // Workspace name
    pub owner_id: Uuid,                            // Owner user ID
    pub members: Vec<WorkspaceMemberRequest>,      // Initial members
}
```

#### `WorkspaceMemberRequest` - Member Addition
```rust
pub struct WorkspaceMemberRequest {
    pub user_id: Uuid,       // User to add
    pub role_name: String,    // Role name (admin, editor, viewer, custom)
}
```

### Result Models

#### `CompleteWorkspaceResult` - Full Workspace Setup
```rust
pub struct CompleteWorkspaceResult {
    pub workspace: Workspace,                    // Created workspace
    pub roles: Vec<Role>,                       // All workspace roles
    pub owner_membership: WorkspaceMember,       // Owner's admin membership
    pub members: Vec<WorkspaceMember>,          // All workspace members
}
```

#### `UserWorkspaceResult` - Combined Registration Result
```rust
pub struct UserWorkspaceResult {
    pub user: User,                             // Created user
    pub workspace: CompleteWorkspaceResult,      // User's workspace setup
}
```

## Validation and Error Handling

### Error Types

The system uses a comprehensive error hierarchy:

```rust
#[derive(Debug, Error)]
pub enum Error {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),           // Database errors

    #[error("Validation error: {0}")]
    Validation(String),                   // Input validation errors

    #[error("Not found: {0}")]
    NotFound(String),                      // Resource not found

    #[error("Access forbidden: {0}")]
    Forbidden(String),                    // Permission denied

    #[error("Conflict: {0}")]
    Conflict(String),                      // Resource conflicts

    #[error("Internal error: {0}")]
    Internal(String),                      // System errors
}
```

### Validation Error Messages

| Validation Type | Error Message | Context |
|-----------------|---------------|---------|
| Password mismatch | "Passwords do not match" | User registration |
| Password length | "Password must be at least 8 characters long" | User registration |
| Empty workspace name | "Workspace name cannot be empty" | Workspace creation |
| Workspace name length | "Workspace name must be less than 100 characters" | Workspace creation |
| Empty role name | "Role name cannot be empty" | Role creation |
| Role name length | "Role name must be less than 100 characters" | Role creation |
| Duplicate role name | "Role '{name}' already exists in this workspace" | Role creation |
| Ownership transfer to self | "Cannot transfer ownership to yourself" | Ownership transfer |
| Not workspace owner | "You are not the owner of this workspace" | Ownership operations |

### Business Logic Validation

#### User Registration Validation
- Password confirmation must match
- Password minimum length: 8 characters
- Email uniqueness (database constraint)
- Valid email format (application-level validation)

#### Workspace Creation Validation
- Name cannot be empty or whitespace-only
- Name maximum length: 100 characters
- Owner must be existing user
- Workspace name uniqueness per owner (optional constraint)

#### Role Management Validation
- Role name uniqueness per workspace
- Role name length limits
- Description length limits
- Valid role assignments for members

#### Ownership Transfer Validation
- Current user must be workspace owner
- Cannot transfer to same user
- New owner automatically gets admin role
- Previous owner retains admin access

## Database Schema and Relationships

### Primary Tables

#### `users` Table
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    full_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### `workspaces` Table
```sql
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### `roles` Table
```sql
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    UNIQUE(workspace_id, name)
);
```

#### `workspace_members` Table
```sql
CREATE TABLE workspace_members (
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (workspace_id, user_id)
);
```

### Relationships and Constraints

#### Foreign Key Relationships
1. **workspaces.owner_id → users.id**: Each workspace has one owner (ON DELETE RESTRICT)
2. **roles.workspace_id → workspaces.id**: Roles belong to workspaces (ON DELETE CASCADE)
3. **workspace_members.workspace_id → workspaces.id**: Members belong to workspaces (ON DELETE CASCADE)
4. **workspace_members.user_id → users.id**: Members are users (ON DELETE CASCADE)
5. **workspace_members.role_id → roles.id**: Members have roles (ON DELETE CASCADE)

#### Unique Constraints
1. **users.email**: Email addresses must be globally unique
2. **roles(workspace_id, name)**: Role names must be unique within each workspace
3. **workspace_members(workspace_id, user_id)**: Users can only have one membership per workspace

#### Cascade Operations
- Deleting a workspace cascades to delete all its roles and members
- Deleting a user removes them from all workspace memberships
- Deleting a role removes the role assignment from all members

### Database Indexes

```sql
-- Performance indexes
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_workspaces_owner_id ON workspaces(owner_id);
CREATE INDEX idx_roles_workspace_id ON roles(workspace_id);
CREATE INDEX idx_workspace_members_workspace_id ON workspace_members(workspace_id);
CREATE INDEX idx_workspace_members_user_id ON workspace_members(user_id);
CREATE INDEX idx_workspace_members_role_id ON workspace_members(role_id);
```

## Usage Examples and Patterns

### Complete User Onboarding

#### Basic User Registration
```rust
use backend::services::users::register_user;
use backend::models::users::RegisterUser;

let register_user = RegisterUser {
    email: "user@example.com".to_string(),
    password: "securepassword123".to_string(),
    confirm_password: "securepassword123".to_string(),
    full_name: Some("John Doe".to_string()),
};

let user = register_user(&mut conn, register_user).await?;
```

#### User Registration with Initial Workspace
```rust
use backend::services::users::register_user_with_workspace;
use backend::models::requests::UserWorkspaceRegistrationRequest;

let registration_request = UserWorkspaceRegistrationRequest {
    email: "user@example.com".to_string(),
    password: "securepassword123".to_string(),
    confirm_password: "securepassword123".to_string(),
    full_name: Some("John Doe".to_string()),
    workspace_name: "John's Workspace".to_string(),
};

let result = register_user_with_workspace(&mut conn, registration_request).await?;
// Returns: UserWorkspaceResult { user, workspace: CompleteWorkspaceResult }
```

### Workspace Creation Patterns

#### Simple Workspace Creation
```rust
use backend::services::workspaces::create_workspace;
use backend::models::requests::CreateWorkspaceRequest;

let workspace_request = CreateWorkspaceRequest {
    name: "Team Workspace".to_string(),
    owner_id: user.id,
};

let result = create_workspace(&mut conn, workspace_request).await?;
// Returns: CompleteWorkspaceResult with default roles + owner as admin
```

#### Workspace Creation with Initial Team
```rust
use backend::services::workspaces::create_workspace_with_members;
use backend::models::requests::{CreateWorkspaceWithMembersRequest, WorkspaceMemberRequest};
use backend::models::roles::{ADMIN_ROLE, EDITOR_ROLE, VIEWER_ROLE};

let workspace_request = CreateWorkspaceWithMembersRequest {
    name: "Project Team".to_string(),
    owner_id: owner_user.id,
    members: vec![
        WorkspaceMemberRequest {
            user_id: editor_user.id,
            role_name: EDITOR_ROLE.to_string(),
        },
        WorkspaceMemberRequest {
            user_id: viewer_user.id,
            role_name: VIEWER_ROLE.to_string(),
        },
    ],
};

let result = create_workspace_with_members(&mut conn, workspace_request).await?;
// Returns: CompleteWorkspaceResult with owner + specified members
```

### Role Management Patterns

#### Using Role Constants
```rust
use backend::models::roles::{ADMIN_ROLE, EDITOR_ROLE, VIEWER_ROLE};

// Always use constants instead of hardcoded strings
let admin_request = WorkspaceMemberRequest {
    user_id: user.id,
    role_name: ADMIN_ROLE.to_string(), // ✅ Correct
};

let wrong_request = WorkspaceMemberRequest {
    user_id: user.id,
    role_name: "admin".to_string(),   // ❌ Avoid hardcoded strings
};
```

#### Creating Custom Roles
```rust
use backend::services::roles::create_single_role;
use backend::models::roles::NewRole;

let custom_role = NewRole {
    workspace_id: workspace.id,
    name: "moderator".to_string(),
    description: Some("Can moderate content but not change settings".to_string()),
};

let role = create_single_role(&mut conn, custom_role).await?;
```

### Access Control Patterns

#### Checking Workspace Access
```rust
use backend::services::workspaces::can_access_workspace;

let can_access = can_access_workspace(&mut conn, workspace_id, user.id).await?;
if can_access {
    // User can access workspace (owner or member)
} else {
    // User has no access to this workspace
}
```

#### Validating Ownership
```rust
use backend::services::workspaces::validate_workspace_ownership;

let is_owner = validate_workspace_ownership(&mut conn, workspace_id, user.id).await?;
if is_owner {
    // User is workspace owner - full permissions
} else {
    // User is not owner - limited permissions
}
```

### Error Handling Patterns

#### Service Layer Error Handling
```rust
match register_user(&mut conn, register_user_data).await {
    Ok(user) => {
        println!("User registered: {}", user.email);
        // Proceed with user creation
    },
    Err(error) => {
        match error {
            Error::Validation(msg) => {
                eprintln!("Validation error: {}", msg);
                // Show user-friendly validation message
            },
            Error::Conflict(msg) => {
                eprintln!("Conflict: {}", msg);
                // Handle duplicate email or other conflicts
            },
            Error::Sqlx(db_error) => {
                eprintln!("Database error: {}", db_error);
                // Handle database connectivity issues
            },
            _ => {
                eprintln!("Unexpected error: {}", error);
                // Handle other errors
            }
        }
    }
}
```

## Security Considerations

### Password Security

#### Argon2 Hashing Configuration
- **Algorithm**: Argon2 (winner of Password Hashing Competition)
- **Salt**: Unique per password using cryptographically secure random generator
- **Memory Cost**: Default Argon2 parameters (configurable if needed)
- **Time Cost**: Default iteration count for balanced security/performance

#### Password Validation
- **Minimum Length**: 8 characters (enforced at service layer)
- **Confirmation Required**: Password must be confirmed during registration
- **No Password Complexity Rules**: Simpler user experience with strong hashing

### Access Control

#### Workspace Isolation
- **Data Separation**: Each workspace's data is completely isolated
- **Role-Based Permissions**: Three-tier role system with clear permission boundaries
- **Ownership Model**: Single owner with full control and transfer capabilities

#### Authentication Flow
1. User registration with password hashing
2. Email uniqueness enforced by database constraints
3. Password verification uses constant-time comparison
4. Workspace access validated through membership checks

### Input Validation

#### Data Sanitization
- **Email Validation**: Format validation + database uniqueness constraint
- **Name Validation**: Length limits and content validation
- **SQL Injection Prevention**: Parameterized queries throughout codebase
- **Cross-Site Scripting**: No direct HTML output in this backend system

#### Constraint Enforcement
- **Database-Level**: All critical constraints enforced at database level
- **Application-Level**: Additional validation for user experience
- **Transaction Safety**: Operations wrapped in transactions for consistency

### Data Protection

#### Sensitive Data Handling
- **Password Hashes**: Never stored in plain text
- **Connection Security**: Assumes TLS for database connections
- **Log Safety**: Sensitive data excluded from logs
- **Memory Management**: Passwords cleared from memory when possible

## Best Practices and Guidelines

### API Usage Patterns

#### Preferred Creation Methods
```rust
// ✅ Preferred: Use comprehensive workspace creation
let result = create_workspace(&mut conn, workspace_request).await?;
// Automatically creates roles and assigns owner as admin

// ❌ Avoid: Manual multi-step creation
let workspace = create_workspace_basic(&mut conn, basic_request).await?;
let roles = create_roles_manually(&mut conn, workspace.id).await?;
let member = add_owner_as_member(&mut conn, workspace.id, owner_id).await?;
```

#### Role Constant Usage
```rust
// ✅ Preferred: Use centralized constants
use backend::models::roles::{ADMIN_ROLE, EDITOR_ROLE, VIEWER_ROLE};

let member_request = WorkspaceMemberRequest {
    user_id: user.id,
    role_name: ADMIN_ROLE.to_string(),
};

// ❌ Avoid: Hardcoded role names
let member_request = WorkspaceMemberRequest {
    user_id: user.id,
    role_name: "admin".to_string(), // Prone to typos, not type-safe
};
```

### Error Handling Guidelines

#### Comprehensive Error Handling
```rust
// ✅ Good: Handle specific error types
match workspace_service(&mut conn, request).await {
    Ok(result) => handle_success(result),
    Err(Error::Validation(msg)) => show_validation_error(msg),
    Err(Error::Conflict(msg)) => handle_conflict(msg),
    Err(Error::NotFound(msg)) => handle_not_found(msg),
    Err(Error::Forbidden(msg)) => handle_forbidden(msg),
    Err(error) => handle_generic_error(error),
}

// ❌ Avoid: Generic error handling
match workspace_service(&mut conn, request).await {
    Ok(result) => handle_success(result),
    Err(_) => panic!("Something went wrong!"), // Too generic
}
```

#### User-Friendly Error Messages
- Map technical errors to user-friendly messages
- Preserve technical details for logging
- Provide actionable error information when possible

### Performance Considerations

#### Database Operations
- **Connection Pooling**: Use efficient connection pooling
- **Batch Operations**: Batch multiple operations when possible
- **Transaction Boundaries**: Keep transactions focused and short
- **Index Utilization**: Ensure queries use appropriate indexes

#### Memory Management
- **Connection Cleanup**: Properly close database connections
- **Large Result Sets**: Use pagination for large queries
- **String Management**: Efficient string handling for large text fields

### Testing Guidelines

#### Test Isolation
- **Unique Prefixes**: Use unique test prefixes for each test
- **Data Cleanup**: Clean up test data after each test
- **Parallel Safety**: Design tests to run safely in parallel
- **Dependency Injection**: Use test databases and mock data

#### Coverage Requirements
- **Service Layer**: Test all business logic and validation rules
- **Database Layer**: Test all CRUD operations and constraints
- **Error Scenarios**: Test both success and failure cases
- **Integration**: Test complete workflows across multiple services

### Development Guidelines

#### Code Organization
- **Module Structure**: Organize by feature (users, workspaces, roles)
- **Layer Separation**: Separate models, services, and queries
- **Consistent Naming**: Use consistent naming conventions
- **Documentation**: Document all public APIs and complex logic

#### Database Migrations
- **Version Control**: Version all database schema changes
- **Backward Compatibility**: Maintain backward compatibility when possible
- **Migration Testing**: Test migrations thoroughly before deployment
- **Rollback Plans**: Have rollback plans for schema changes

## Conclusion

This users, roles, and workspaces system provides a comprehensive foundation for multi-tenant applications with:

1. **Robust Security**: Industry-standard password hashing and access control
2. **Flexible Architecture**: Support for custom roles and complex workspace hierarchies
3. **Developer-Friendly**: Simplified APIs and comprehensive error handling
4. **Scalable Design**: Efficient database schema and relationship management
5. **Production-Ready**: Comprehensive validation, constraints, and error handling

The system is designed to be extensible and maintainable, with clear separation of concerns and well-documented APIs. Following the best practices and guidelines outlined in this documentation will ensure consistent, secure, and efficient use of the system.