use buildscale::{
    services::workspace_members::{create_workspace_member, list_members, add_member_by_email, update_member_role, remove_member},
    models::workspace_members::{AddMemberRequest, UpdateMemberRoleRequest},
};
use crate::common::database::TestApp;
use buildscale::models::roles::MEMBER_ROLE;

#[tokio::test]
async fn test_workspace_member_creation_success() {
    let test_app = TestApp::new("test_workspace_member_creation_success").await;
    let mut conn = test_app.get_connection().await;

    // Create a complete test scenario
    let (_user, workspace, role, _) = test_app.create_complete_test_scenario().await.unwrap();

    // Create a new user to add as member
    let user_data = test_app.generate_test_user();
    let new_user = buildscale::services::users::register_user(&mut conn, user_data).await.unwrap();

    // Add the new user as a workspace member
    let member_data = test_app.generate_test_workspace_member(workspace.id, new_user.id, role.id);
    let result = create_workspace_member(&mut conn, member_data).await;
    assert!(result.is_ok(), "Workspace member creation should succeed");

    let created_member = result.unwrap();
    assert_eq!(created_member.workspace_id, workspace.id, "Workspace ID should match");
    assert_eq!(created_member.user_id, new_user.id, "User ID should match");
    assert_eq!(created_member.role_id, role.id, "Role ID should match");

    // Verify member count increased
    let member_count = test_app.count_workspace_members(workspace.id).await.unwrap();
    assert_eq!(member_count, 2, "Should have exactly two members");

    // Verify membership
    let is_member = test_app.is_workspace_member(workspace.id, new_user.id).await.unwrap();
    assert!(is_member, "New user should be a workspace member");
}

#[tokio::test]
async fn test_workspace_member_creation_duplicate_validation() {
    let test_app = TestApp::new("test_workspace_member_creation_duplicate_validation").await;
    let mut conn = test_app.get_connection().await;

    // Create a complete test scenario
    let (user, workspace, role, _) = test_app.create_complete_test_scenario().await.unwrap();

    // Try to add the same user as a member again (should fail)
    let duplicate_member_data = test_app.generate_test_workspace_member(workspace.id, user.id, role.id);
    let result = create_workspace_member(&mut conn, duplicate_member_data).await;
    assert!(
        result.is_err(),
        "Adding duplicate member should fail"
    );

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("already a member"),
        "Error message should mention already a member: {}",
        error_message
    );
}

#[tokio::test]
async fn test_workspace_member_removal() {
    let test_app = TestApp::new("test_workspace_member_removal").await;
    let mut conn = test_app.get_connection().await;

    // Create a complete test scenario (owner is automatically created as member)
    let (_owner, workspace, role, _) = test_app.create_complete_test_scenario().await.unwrap();

    // Create a separate user to add as a member (not the owner)
    let user_data = test_app.generate_test_user();
    let user = buildscale::services::users::register_user(&mut conn, user_data).await.unwrap();

    // Add the user as a workspace member
    let member_data = test_app.generate_test_workspace_member(workspace.id, user.id, role.id);
    buildscale::services::workspace_members::create_workspace_member(&mut conn, member_data).await.unwrap();

    // Verify member exists
    assert!(
        test_app.is_workspace_member(workspace.id, user.id).await.unwrap(),
        "User should be a workspace member before removal"
    );

    // Remove the member (not the owner)
    let result = buildscale::services::workspace_members::remove_workspace_member(
        &mut conn,
        workspace.id,
        user.id,
    ).await;
    assert!(result.is_ok(), "Member removal should succeed");

    // Verify member no longer exists
    let is_member = test_app.is_workspace_member(workspace.id, user.id).await.unwrap();
    assert!(!is_member, "User should no longer be a workspace member");

    // Verify member count decreased (should be back to 1 - just the owner)
    let member_count = test_app.count_workspace_members(workspace.id).await.unwrap();
    assert_eq!(member_count, 1, "Should have exactly one member (the owner)");
}

#[tokio::test]
async fn test_list_members_detailed() {
    let test_app = TestApp::new("test_list_members_detailed").await;
    let mut conn = test_app.get_connection().await;

    let (user, workspace, _role, _) = test_app.create_complete_test_scenario().await.unwrap();

    let result = list_members(&mut conn, workspace.id, user.id).await;
    assert!(result.is_ok());
    
    let members = result.unwrap();
    assert!(!members.is_empty());
    assert_eq!(members[0].email, user.email);
}

#[tokio::test]
async fn test_add_member_by_email_success() {
    let test_app = TestApp::new("test_add_member_by_email_success").await;
    let mut conn = test_app.get_connection().await;

    // Use comprehensive creation to get default roles
    let user_data = test_app.generate_test_user();
    let owner = buildscale::services::users::register_user(&mut conn, user_data).await.unwrap();
    let workspace_result = buildscale::services::workspaces::create_workspace(
        &mut conn,
        buildscale::models::requests::CreateWorkspaceRequest {
            name: "Default Roles Workspace".to_string(),
            owner_id: owner.id,
        },
    ).await.unwrap();
    
    let workspace = workspace_result.workspace;
    let owner_id = workspace.owner_id;

    // New user to add
    let new_user_data = test_app.generate_test_user();
    let new_user_email = new_user_data.email.clone();
    buildscale::services::users::register_user(&mut conn, new_user_data).await.unwrap();

    let request = AddMemberRequest {
        email: new_user_email.clone(),
        role_name: MEMBER_ROLE.to_string(),
    };

    let result = add_member_by_email(&mut conn, workspace.id, owner_id, request).await;
    assert!(result.is_ok());
    
    let member = result.unwrap();
    assert_eq!(member.email, new_user_email);
    assert_eq!(member.role_name, MEMBER_ROLE);
}

#[tokio::test]
async fn test_update_member_role_success() {
    let test_app = TestApp::new("test_update_member_role_success").await;
    let mut conn = test_app.get_connection().await;

    // Create workspace with default roles
    let user_data = test_app.generate_test_user();
    let owner = buildscale::services::users::register_user(&mut conn, user_data).await.unwrap();
    let workspace_result = buildscale::services::workspaces::create_workspace(
        &mut conn,
        buildscale::models::requests::CreateWorkspaceRequest {
            name: "Update Role Workspace".to_string(),
            owner_id: owner.id,
        },
    ).await.unwrap();
    let workspace = workspace_result.workspace;

    // Add a member
    let new_user_data = test_app.generate_test_user();
    let new_user = buildscale::services::users::register_user(&mut conn, new_user_data).await.unwrap();
    
    add_member_by_email(&mut conn, workspace.id, owner.id, AddMemberRequest {
        email: new_user.email.clone(),
        role_name: "viewer".to_string(),
    }).await.unwrap();

    // Update to editor
    let result = update_member_role(
        &mut conn,
        workspace.id,
        new_user.id,
        owner.id,
        UpdateMemberRoleRequest { role_name: "editor".to_string() }
    ).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().role_name, "editor");
}

#[tokio::test]
async fn test_remove_member_self_success() {
    let test_app = TestApp::new("test_remove_member_self_success").await;
    let mut conn = test_app.get_connection().await;

    let user_data = test_app.generate_test_user();
    let owner = buildscale::services::users::register_user(&mut conn, user_data).await.unwrap();
    let workspace_result = buildscale::services::workspaces::create_workspace(
        &mut conn,
        buildscale::models::requests::CreateWorkspaceRequest {
            name: "Leave Workspace".to_string(),
            owner_id: owner.id,
        },
    ).await.unwrap();
    let workspace = workspace_result.workspace;

    let new_user_data = test_app.generate_test_user();
    let new_user = buildscale::services::users::register_user(&mut conn, new_user_data).await.unwrap();
    
    add_member_by_email(&mut conn, workspace.id, owner.id, AddMemberRequest {
        email: new_user.email.clone(),
        role_name: "member".to_string(),
    }).await.unwrap();

    // Member leaves (removes self)
    let result = remove_member(&mut conn, workspace.id, new_user.id, new_user.id).await;
    assert!(result.is_ok());
    
    let is_member = buildscale::queries::workspace_members::is_workspace_member(&mut conn, workspace.id, new_user.id).await.unwrap();
    assert!(!is_member);
}
