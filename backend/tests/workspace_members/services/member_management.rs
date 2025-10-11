use backend::{
    services::workspace_members::create_workspace_member,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_workspace_member_creation_success() {
    let test_app = TestApp::new("test_workspace_member_creation_success").await;
    let mut conn = test_app.get_connection().await;

    // Create a complete test scenario
    let (user, workspace, role, _) = test_app.create_complete_test_scenario().await.unwrap();

    // Create a new user to add as member
    let user_data = test_app.generate_test_user();
    let new_user = backend::services::users::register_user(&mut conn, user_data).await.unwrap();

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
    let (owner, workspace, role, _) = test_app.create_complete_test_scenario().await.unwrap();

    // Create a separate user to add as a member (not the owner)
    let user_data = test_app.generate_test_user();
    let user = backend::services::users::register_user(&mut conn, user_data).await.unwrap();

    // Add the user as a workspace member
    let member_data = test_app.generate_test_workspace_member(workspace.id, user.id, role.id);
    backend::services::workspace_members::create_workspace_member(&mut conn, member_data).await.unwrap();

    // Verify member exists
    assert!(
        test_app.is_workspace_member(workspace.id, user.id).await.unwrap(),
        "User should be a workspace member before removal"
    );

    // Remove the member (not the owner)
    let result = backend::services::workspace_members::remove_workspace_member(
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