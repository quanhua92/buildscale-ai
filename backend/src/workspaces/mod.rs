//! Workspaces module - Workspace management with RBAC
//!
//! This module provides a layered architecture for workspace management:
//! - `models`: Workspace, member, role, invitation, and permission types
//! - `queries`: Database operations for workspace domain
//! - `services`: Business logic for workspace management

pub mod models;
pub mod queries;
pub mod services;

// Re-export commonly used types at module level for convenience
pub use models::{
    Workspace, NewWorkspace, UpdateWorkspace,
    WorkspaceMember, NewWorkspaceMember, UpdateWorkspaceMember, WorkspaceMemberDetailed,
    Role, NewRole, WorkspaceRole, ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE,
    WorkspaceInvitation, NewWorkspaceInvitation,
};
