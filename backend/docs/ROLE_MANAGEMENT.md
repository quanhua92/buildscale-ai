← [Back to Index](./README.md) | **Related**: [User Management](./USER_WORKSPACE_MANAGEMENT.md), [Invitations](./WORKSPACE_INVITATIONS.md)

# Role-Based Access Control (RBAC)

Multi-tier role hierarchy with comprehensive permissions across workspace, content, and member management categories.

## Role Hierarchy

| Role | Description | Scope |
|------|-------------|-------|
| **Admin** | Full workspace control | All permissions |
| **Editor** | Content creation and editing | Content management + workspace access |
| **Member** | Basic content participation | Own content + workspace access |
| **Viewer** | Read-only access | View-only permissions |

## Permission System

### Permission Categories

#### Workspace Permissions
```rust
workspace_permissions::READ                    // View workspace
workspace_permissions::WRITE                  // Modify workspace
workspace_permissions::DELETE                // Delete workspace
workspace_permissions::MANAGE_MEMBERS        // Manage members
workspace_permissions::MANAGE_SETTINGS      // Manage settings
workspace_permissions::INVITE_MEMBERS        // Invite members
workspace_permissions::VIEW_ACTIVITY_LOG     // View activity log
workspace_permissions::EXPORT_DATA          // Export data
```

#### Content Permissions
```rust
content_permissions::CREATE                 // Create content
content_permissions::READ_OWN               // Read own content
content_permissions::READ_ALL               // Read all content
content_permissions::UPDATE_OWN             // Update own content
content_permissions::UPDATE_ALL             // Update any content
content_permissions::DELETE_OWN             // Delete own content
content_permissions::DELETE_ALL             // Delete any content
content_permissions::COMMENT                // Comment on content
```

#### Member Permissions
```rust
member_permissions::ADD_MEMBERS            // Add members
member_permissions::REMOVE_MEMBERS         // Remove members
member_permissions::UPDATE_ROLES           // Update roles
member_permissions::VIEW_MEMBERS           // View members
```

## Role Permission Matrix

| Permission | Admin | Editor | Member | Viewer |
|------------|--------|--------|--------|--------|
| **Workspace** |
| `workspace:read` | ✓ | ✓ | ✓ | ✓ |
| `workspace:write` | ✓ | ✓ | ✗ | ✗ |
| `workspace:delete` | ✓ | ✗ | ✗ | ✗ |
| `workspace:manage_members` | ✓ | ✗ | ✗ | ✗ |
| `workspace:manage_settings` | ✓ | ✗ | ✗ | ✗ |
| `workspace:invite_members` | ✓ | ✗ | ✗ | ✗ |
| `workspace:view_activity_log` | ✓ | ✗ | ✗ | ✗ |
| `workspace:export_data` | ✓ | ✓ | ✗ | ✗ |
| **Content** |
| `content:create` | ✓ | ✓ | ✓ | ✗ |
| `content:read_own` | ✓ | ✓ | ✓ | ✓ |
| `content:read_all` | ✓ | ✓ | ✓ | ✓ |
| `content:update_own` | ✓ | ✓ | ✓ | ✗ |
| `content:update_all` | ✓ | ✓ | ✗ | ✗ |
| `content:delete_own` | ✓ | ✓ | ✓ | ✗ |
| `content:delete_all` | ✓ | ✓ | ✗ | ✗ |
| `content:comment` | ✓ | ✓ | ✓ | ✗ |
| **Members** |
| `members:add` | ✓ | ✗ | ✗ | ✗ |
| `members:remove` | ✓ | ✗ | ✗ | ✗ |
| `members:update_roles` | ✓ | ✗ | ✗ | ✗ |
| `members:view` | ✓ | ✓ | ✓ | ✓ |

## Core APIs

### Role Management
```rust
// Create default 4-tier role system for workspace
pub async fn create_default_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>

// Create custom workspace-specific role
pub async fn create_single_role(conn: &mut DbConn, new_role: NewRole) -> Result<Role>

// Role lookup and listing
pub async fn get_role(conn: &mut DbConn, id: Uuid) -> Result<Role>
pub async fn get_role_by_name(
    conn: &mut DbConn,
    workspace_id: Uuid,
    role_name: &str
) -> Result<Role>

pub async fn list_workspace_roles(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<Role>>
```

### Permission Validation
```rust
// Check if role has specific permission
pub fn role_has_permission(role: &str, permission: &str) -> bool

// Check if role has any of specified permissions (OR logic)
pub fn role_has_any_permission(role: &str, permissions: &[&str]) -> bool

// Check if role has all specified permissions (AND logic)
pub fn role_has_all_permissions(role: &str, permissions: &[&str]) -> bool

// Get all permissions for a role
pub fn get_role_permissions(role: &str) -> Vec<&'static str>

// Validate permission exists in system
pub fn is_valid_permission(permission: &str) -> bool
```

### Member Permission Validation
```rust
// Validate user has specific permission in workspace
pub async fn validate_workspace_permission(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    permission: &str,
) -> Result<()>
```

## Role Constants

```rust
// Type-safe role names
use backend::models::roles::{ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE};

pub const ADMIN_ROLE: &str = "admin";      // Full workspace control
pub const EDITOR_ROLE: &str = "editor";     // Content creation and management
pub const MEMBER_ROLE: &str = "member";     // Basic content participation
pub const VIEWER_ROLE: &str = "viewer";     // Read-only access

// Type-safe enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceRole {
    Admin, Editor, Member, Viewer,
}
```

## Data Models

```rust
pub struct Role {
    pub id: Uuid,                  // Role ID (UUID v7)
    pub workspace_id: Uuid,          // Workspace ID
    pub name: String,               // Role name
    pub description: Option<String>,   // Role description
}

pub struct NewRole {
    pub workspace_id: Uuid,          // Target workspace
    pub name: String,               // Role name
    pub description: Option<String>,   // Optional description
}
```

## Usage Examples

### Permission Validation
```rust
use backend::models::permissions::*;

// Check role has permission
if PermissionValidator::role_has_permission("editor", "content:create") {
    // Allow content creation
}

// Validate multiple permissions
let required_perms = vec![
    workspace_permissions::READ,
    content_permissions::CREATE
];

if PermissionValidator::role_has_all_permissions("member", &required_perms) {
    // User has all required permissions
}

// Validate workspace permission for user
let can_invite = validate_workspace_permission(
    &mut conn,
    workspace_id,
    user_id,
    workspace_permissions::INVITE_MEMBERS,
).await.is_ok();
```

### Role Management
```rust
// Create default roles (auto-called during workspace creation)
let roles = create_default_roles(&mut conn, workspace_id).await?;
// Returns: [admin, editor, member, viewer]

// Create custom role
let new_role = NewRole {
    workspace_id: workspace.id,
    name: "moderator".to_string(),
    description: Some("Custom role description".to_string()),
};

let role = create_single_role(&mut conn, new_role).await?;
```

## Database Schema

```sql
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    UNIQUE(workspace_id, name)
);

-- Performance indexes
CREATE INDEX idx_roles_workspace_id ON roles(workspace_id);
CREATE INDEX idx_roles_name ON roles(name);
```

## Common Permission Sets

```rust
// Frequently used combinations
basic_workspace_access()       -> vec!["workspace:read"]
content_management()           -> vec!["workspace:read", "workspace:write", "content:create", ...]
member_management()           -> vec!["workspace:manage_members", "members:add", ...]
workspace_administration()    -> vec!["workspace:manage_members", "workspace:manage_settings", ...]
```

## Related Documentation

- **[User & Workspace Management](./USER_WORKSPACE_MANAGEMENT.md)** - User and workspace operations with role assignments
- **[Authentication & Security](./AUTHENTICATION.md)** - User authentication and session management
- **[Architecture Overview](./ARCHITECTURE.md)** - System design and permission architecture
- **[API Guide](./API_GUIDE.md)** - Complete API reference with error handling
- **[Workspace Invitations](./WORKSPACE_INVITATIONS.md)** - Role-based invitation system

## For Developers

### Finding Current Permission Counts
```bash
# Count permissions in the source code
grep -c "pub const.*:" src/models/permissions.rs

# List workspace permissions
grep "pub const" src/models/permissions.rs | head -8

# List content permissions
grep "pub const" src/models/permissions.rs | head -16 | tail -8

# List member permissions
grep "pub const" src/models/permissions.rs | tail -4

# Count total permissions in ALL_PERMISSIONS array
grep -A 50 "pub const ALL_PERMISSIONS" src/models/permissions.rs | grep -c "pub const"
```

### Understanding Role Permissions
The permission system is defined in `src/models/permissions.rs`:
- `ALL_PERMISSIONS`: Array containing all available permissions
- `ROLE_PERMISSIONS`: HashMap mapping roles to their permission sets
- `PermissionValidator`: Utility functions for permission checking

### Adding New Permissions
1. Add permission constant to appropriate module (workspace/content/member)
2. Add to `ALL_PERMISSIONS` array
3. Update role assignments in `ROLE_PERMISSIONS`
4. Add tests for new permission
5. Update configuration documentation if needed

### Current Permission Structure
- **Workspace Permissions**: Workspace management and administration
- **Content Permissions**: Content creation, modification, and access
- **Member Permissions**: Member management and role assignments

Total permissions and role assignments can change as the system evolves. Always check the source code for current values.