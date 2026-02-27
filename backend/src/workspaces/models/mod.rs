//! Workspace models - Data structures for workspace domain

pub mod invitation;
pub mod member;
pub mod permission;
pub mod role;
pub mod workspace;

// Re-export commonly used types
pub use invitation::{
    WorkspaceInvitation, NewWorkspaceInvitation, UpdateWorkspaceInvitation,
    CreateInvitationRequest, CreateInvitationResponse, AcceptInvitationRequest,
    AcceptInvitationResponse, RevokeInvitationRequest, InvitationSummary,
    InvitationStatus, InvitationValidator, InvitationUtils,
    INVITATION_STATUS_PENDING, INVITATION_STATUS_ACCEPTED, INVITATION_STATUS_EXPIRED,
    INVITATION_STATUS_REVOKED, VALID_INVITATION_STATUSES,
    DEFAULT_INVITATION_EXPIRATION_HOURS, MAX_INVITATION_EXPIRATION_HOURS,
};
pub use member::{
    WorkspaceMember, NewWorkspaceMember, UpdateWorkspaceMember, WorkspaceMemberDetailed,
    AddMemberRequest, UpdateMemberRoleRequest,
};
pub use permission::{
    PermissionValidator, PermissionCategory,
    workspace_permissions, content_permissions, member_permissions, ALL_PERMISSIONS,
};
pub use role::{
    Role, NewRole, UpdateRole, WorkspaceRole,
    ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE, DEFAULT_ROLES, descriptions,
};
pub use workspace::{Workspace, NewWorkspace, UpdateWorkspace};
