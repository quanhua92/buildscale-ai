use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use crate::models::roles::{ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE};

/// Workspace permission constants
pub mod workspace_permissions {
    pub const READ: &str = "workspace:read";
    pub const WRITE: &str = "workspace:write";
    pub const DELETE: &str = "workspace:delete";
    pub const MANAGE_MEMBERS: &str = "workspace:manage_members";
    pub const MANAGE_SETTINGS: &str = "workspace:manage_settings";
    pub const INVITE_MEMBERS: &str = "workspace:invite_members";
    pub const VIEW_ACTIVITY_LOG: &str = "workspace:view_activity_log";
    pub const EXPORT_DATA: &str = "workspace:export_data";
}

/// Content permission constants
pub mod content_permissions {
    pub const CREATE: &str = "content:create";
    pub const READ_OWN: &str = "content:read_own";
    pub const READ_ALL: &str = "content:read_all";
    pub const UPDATE_OWN: &str = "content:update_own";
    pub const UPDATE_ALL: &str = "content:update_all";
    pub const DELETE_OWN: &str = "content:delete_own";
    pub const DELETE_ALL: &str = "content:delete_all";
    pub const COMMENT: &str = "content:comment";
}

/// Member management permission constants
pub mod member_permissions {
    pub const ADD_MEMBERS: &str = "members:add";
    pub const REMOVE_MEMBERS: &str = "members:remove";
    pub const UPDATE_ROLES: &str = "members:update_roles";
    pub const VIEW_MEMBERS: &str = "members:view";
}

/// All available permissions
pub const ALL_PERMISSIONS: &[&str] = &[
    // Workspace permissions
    workspace_permissions::READ,
    workspace_permissions::WRITE,
    workspace_permissions::DELETE,
    workspace_permissions::MANAGE_MEMBERS,
    workspace_permissions::MANAGE_SETTINGS,
    workspace_permissions::INVITE_MEMBERS,
    workspace_permissions::VIEW_ACTIVITY_LOG,
    workspace_permissions::EXPORT_DATA,
    // Content permissions
    content_permissions::CREATE,
    content_permissions::READ_OWN,
    content_permissions::READ_ALL,
    content_permissions::UPDATE_OWN,
    content_permissions::UPDATE_ALL,
    content_permissions::DELETE_OWN,
    content_permissions::DELETE_ALL,
    content_permissions::COMMENT,
    // Member permissions
    member_permissions::ADD_MEMBERS,
    member_permissions::REMOVE_MEMBERS,
    member_permissions::UPDATE_ROLES,
    member_permissions::VIEW_MEMBERS,
];

/// Role-to-permission mappings
///
/// This static mapping defines which permissions each role has.
/// The hierarchy is: Admin > Editor > Member > Viewer
pub static ROLE_PERMISSIONS: LazyLock<HashMap<&'static str, HashSet<&'static str>>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    // Admin: All permissions
    map.insert(ADMIN_ROLE, ALL_PERMISSIONS.iter().copied().collect::<HashSet<_>>());

    // Editor: Read, write, and manage most content, but not workspace administration
    let editor_permissions = vec![
        workspace_permissions::READ,
        workspace_permissions::WRITE,
        member_permissions::VIEW_MEMBERS,
        workspace_permissions::EXPORT_DATA,
        content_permissions::CREATE,
        content_permissions::READ_OWN,
        content_permissions::READ_ALL,
        content_permissions::UPDATE_OWN,
        content_permissions::UPDATE_ALL,
        content_permissions::DELETE_OWN,
        content_permissions::DELETE_ALL,
        content_permissions::COMMENT,
    ].into_iter().collect::<HashSet<_>>();
    map.insert(EDITOR_ROLE, editor_permissions);

    // Member: Can create and edit own content, participate
    let member_permissions = vec![
        workspace_permissions::READ,
        member_permissions::VIEW_MEMBERS,
        content_permissions::CREATE,
        content_permissions::READ_OWN,
        content_permissions::READ_ALL,
        content_permissions::UPDATE_OWN,
        content_permissions::DELETE_OWN,
        content_permissions::COMMENT,
    ].into_iter().collect::<HashSet<_>>();
    map.insert(MEMBER_ROLE, member_permissions);

    // Viewer: Read-only access
    let viewer_permissions = vec![
        workspace_permissions::READ,
        content_permissions::READ_OWN,
        content_permissions::READ_ALL,
    ].into_iter().collect::<HashSet<_>>();
    map.insert(VIEWER_ROLE, viewer_permissions);

    map
});

/// Permission categories for organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionCategory {
    Workspace,
    Content,
    Members,
}

impl PermissionCategory {
    /// Get all permissions in a category
    pub fn permissions(&self) -> Vec<&'static str> {
        match self {
            PermissionCategory::Workspace => vec![
                workspace_permissions::READ,
                workspace_permissions::WRITE,
                workspace_permissions::DELETE,
                workspace_permissions::MANAGE_MEMBERS,
                workspace_permissions::MANAGE_SETTINGS,
                workspace_permissions::INVITE_MEMBERS,
                workspace_permissions::VIEW_ACTIVITY_LOG,
                workspace_permissions::EXPORT_DATA,
            ],
            PermissionCategory::Content => vec![
                content_permissions::CREATE,
                content_permissions::READ_OWN,
                content_permissions::READ_ALL,
                content_permissions::UPDATE_OWN,
                content_permissions::UPDATE_ALL,
                content_permissions::DELETE_OWN,
                content_permissions::DELETE_ALL,
                content_permissions::COMMENT,
            ],
            PermissionCategory::Members => vec![
                member_permissions::ADD_MEMBERS,
                member_permissions::REMOVE_MEMBERS,
                member_permissions::UPDATE_ROLES,
                member_permissions::VIEW_MEMBERS,
            ],
        }
    }
}

/// Permission validation utilities
pub struct PermissionValidator;

impl PermissionValidator {
    /// Check if a role has a specific permission
    pub fn role_has_permission(role: &str, permission: &str) -> bool {
        ROLE_PERMISSIONS
            .get(role)
            .map(|permissions| permissions.contains(permission))
            .unwrap_or(false)
    }

    /// Check if a role has any of the specified permissions (OR logic)
    pub fn role_has_any_permission(role: &str, permissions: &[&str]) -> bool {
        ROLE_PERMISSIONS
            .get(role)
            .map(|role_permissions| permissions.iter().any(|p| role_permissions.contains(p)))
            .unwrap_or(false)
    }

    /// Check if a role has all of the specified permissions (AND logic)
    pub fn role_has_all_permissions(role: &str, permissions: &[&str]) -> bool {
        ROLE_PERMISSIONS
            .get(role)
            .map(|role_permissions| permissions.iter().all(|p| role_permissions.contains(p)))
            .unwrap_or(false)
    }

    /// Get all permissions for a role
    pub fn get_role_permissions(role: &str) -> Vec<&'static str> {
        ROLE_PERMISSIONS
            .get(role)
            .map(|permissions| permissions.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Check if a permission exists in the system
    pub fn is_valid_permission(permission: &str) -> bool {
        ALL_PERMISSIONS.contains(&permission)
    }
}

/// Common permission combinations for frequent checks
pub mod common_permission_sets {
    use super::*;

    /// Permissions required for basic workspace access
    pub fn basic_workspace_access() -> Vec<&'static str> {
        vec![workspace_permissions::READ]
    }

    /// Permissions required for content management
    pub fn content_management() -> Vec<&'static str> {
        vec![
            workspace_permissions::READ,
            workspace_permissions::WRITE,
            content_permissions::CREATE,
            content_permissions::UPDATE_OWN,
            content_permissions::DELETE_OWN,
        ]
    }

    /// Permissions required for member management
    pub fn member_management() -> Vec<&'static str> {
        vec![
            workspace_permissions::MANAGE_MEMBERS,
            member_permissions::ADD_MEMBERS,
            member_permissions::REMOVE_MEMBERS,
            member_permissions::UPDATE_ROLES,
            member_permissions::VIEW_MEMBERS,
        ]
    }

    /// Permissions required for workspace administration
    pub fn workspace_administration() -> Vec<&'static str> {
        vec![
            workspace_permissions::MANAGE_MEMBERS,
            workspace_permissions::MANAGE_SETTINGS,
            workspace_permissions::INVITE_MEMBERS,
            workspace_permissions::VIEW_ACTIVITY_LOG,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_has_all_permissions() {
        for permission in ALL_PERMISSIONS {
            assert!(
                PermissionValidator::role_has_permission(ADMIN_ROLE, permission),
                "Admin should have permission: {}",
                permission
            );
        }
    }

    #[test]
    fn test_editor_permissions() {
        assert!(PermissionValidator::role_has_permission(EDITOR_ROLE, workspace_permissions::WRITE));
        assert!(PermissionValidator::role_has_permission(EDITOR_ROLE, content_permissions::CREATE));
        assert!(!PermissionValidator::role_has_permission(EDITOR_ROLE, workspace_permissions::MANAGE_MEMBERS));
    }

    #[test]
    fn test_member_permissions() {
        assert!(PermissionValidator::role_has_permission(MEMBER_ROLE, content_permissions::CREATE));
        assert!(PermissionValidator::role_has_permission(MEMBER_ROLE, content_permissions::UPDATE_OWN));
        assert!(!PermissionValidator::role_has_permission(MEMBER_ROLE, workspace_permissions::MANAGE_SETTINGS));
    }

    #[test]
    fn test_viewer_permissions() {
        assert!(PermissionValidator::role_has_permission(VIEWER_ROLE, workspace_permissions::READ));
        assert!(PermissionValidator::role_has_permission(VIEWER_ROLE, content_permissions::READ_ALL));
        assert!(!PermissionValidator::role_has_permission(VIEWER_ROLE, content_permissions::CREATE));
    }

    #[test]
    fn test_permission_validation_utilities() {
        assert!(PermissionValidator::is_valid_permission(workspace_permissions::READ));
        assert!(!PermissionValidator::is_valid_permission("invalid:permission"));
    }

    #[test]
    fn test_common_permission_sets() {
        let basic_access = common_permission_sets::basic_workspace_access();
        assert!(PermissionValidator::role_has_all_permissions(VIEWER_ROLE, &basic_access));

        let content_mgmt = common_permission_sets::content_management();
        assert!(PermissionValidator::role_has_all_permissions(EDITOR_ROLE, &content_mgmt));
    }
}