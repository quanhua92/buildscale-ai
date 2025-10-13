use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Default workspace roles
pub const ADMIN_ROLE: &str = "admin";
pub const EDITOR_ROLE: &str = "editor";
pub const MEMBER_ROLE: &str = "member";
pub const VIEWER_ROLE: &str = "viewer";

/// All default roles in order of hierarchy (admin > editor > member > viewer)
pub const DEFAULT_ROLES: [&str; 4] = [ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE];

/// Enum representing workspace roles with type safety
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceRole {
    Admin,
    Editor,
    Member,
    Viewer,
}

impl WorkspaceRole {
    /// Get the string representation of the role
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkspaceRole::Admin => ADMIN_ROLE,
            WorkspaceRole::Editor => EDITOR_ROLE,
            WorkspaceRole::Member => MEMBER_ROLE,
            WorkspaceRole::Viewer => VIEWER_ROLE,
        }
    }

    /// Get the role name as a String
    pub fn name(&self) -> String {
        self.as_str().to_string()
    }

    /// Create a WorkspaceRole from a string
    pub fn from_str(role: &str) -> Option<Self> {
        match role {
            ADMIN_ROLE => Some(WorkspaceRole::Admin),
            EDITOR_ROLE => Some(WorkspaceRole::Editor),
            MEMBER_ROLE => Some(WorkspaceRole::Member),
            VIEWER_ROLE => Some(WorkspaceRole::Viewer),
            _ => None,
        }
    }
}

impl From<WorkspaceRole> for String {
    fn from(role: WorkspaceRole) -> Self {
        role.as_str().to_string()
    }
}

impl AsRef<str> for WorkspaceRole {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for WorkspaceRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Role descriptions for default roles
pub mod descriptions {
    use super::{ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE};

    pub const ADMIN: &str = "Full administrative access to workspace";
    pub const EDITOR: &str = "Can create and edit any content";
    pub const MEMBER: &str = "Can create and edit their own content, comment, and participate in discussions";
    pub const VIEWER: &str = "Read-only access to workspace";

    /// Get description for a role name
    pub fn for_role(role_name: &str) -> &'static str {
        match role_name {
            ADMIN_ROLE => ADMIN,
            EDITOR_ROLE => EDITOR,
            MEMBER_ROLE => MEMBER,
            VIEWER_ROLE => VIEWER,
            _ => "Custom role",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRole {
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub description: Option<String>,
}
