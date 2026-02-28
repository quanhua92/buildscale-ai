//! Workspace queries - Database operations for workspace domain

pub mod invitations;
pub mod members;
pub mod roles;
pub mod workspaces;

// Re-export query functions
pub use invitations::{
    create_invitation, get_invitation_by_id, get_invitation_by_id_optional,
    get_invitation_by_token, list_invitations_by_workspace, list_invitations_by_email,
    list_invitations_by_inviter, update_invitation, update_invitation_status_by_token,
    delete_invitation, delete_invitation_by_token, delete_invitations_by_workspace,
    delete_expired_invitations, check_existing_pending_invitation, count_invitations_by_status,
    get_recent_invitations, get_invitations_expiring_soon,
};
pub use members::{
    create_workspace_member, get_workspace_member, get_workspace_member_optional,
    list_workspace_members, list_user_workspaces, list_workspace_members_all,
    update_workspace_member, delete_workspace_member, delete_workspace_members_by_workspace,
    delete_workspace_members_by_user, is_workspace_member, get_workspace_ids_by_user,
    list_workspace_members_detailed, get_workspace_member_detailed,
};
pub use roles::{
    create_role, get_role_by_id, get_role_by_id_optional, get_role_by_workspace_and_name,
    list_roles_by_workspace, list_roles, update_role, delete_role, delete_roles_by_workspace,
};
pub use workspaces::{
    create_workspace, get_workspace_by_id, get_workspace_by_id_optional,
    get_workspaces_by_owner, list_workspaces, update_workspace, delete_workspace,
    is_workspace_owner, get_workspaces_by_user_membership,
};
