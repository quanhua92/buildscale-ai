//! Workspace services - Business logic for workspace domain

pub mod invitations;
pub mod members;
pub mod roles;
pub mod workspaces;

// Re-export service functions
pub use invitations::{
    create_invitation, get_invitation_by_token, list_workspace_invitations,
    list_user_sent_invitations, list_email_invitations, accept_invitation,
    revoke_invitation, delete_invitation, resend_invitation, cleanup_expired_invitations,
    get_invitations_expiring_soon, get_workspace_invitation_stats, bulk_create_invitations,
};
pub use members::{
    list_workspace_members, list_user_workspaces, get_workspace_member,
    get_workspace_member_optional, is_workspace_member, update_workspace_member,
    remove_workspace_member, create_workspace_member, validate_workspace_permission,
    require_workspace_permission, validate_any_workspace_permission,
    validate_all_workspace_permissions, get_user_workspace_permissions,
    list_members, get_my_membership, add_member_by_email, update_member_role, remove_member,
};
pub use roles::{
    create_default_roles, create_single_role, get_role, list_workspace_roles, get_role_by_name,
};
pub use workspaces::{
    create_workspace, create_workspace_with_members, update_workspace_owner,
    update_workspace, get_workspace, list_workspaces,
    validate_workspace_ownership, can_access_workspace, check_workspace_access, delete_workspace,
};
// Re-export with alias to avoid conflict with members::list_user_workspaces
pub use workspaces::list_user_workspaces as list_user_workspaces_detail;
