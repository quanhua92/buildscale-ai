use backend::{
    models::workspace_members::NewWorkspaceMember,
    queries::workspace_members::create_workspace_member,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_create_workspace_member_query() {
    let test_app = TestApp::new("test_create_workspace_member_query").await;
    let mut conn = test_app.get_connection().await;

    // Create a complete test scenario
    let (user, workspace, role, _) = test_app.create_complete_test_scenario().await.unwrap();

    // Create a NewWorkspaceMember manually (bypassing service layer)
    let new_member = NewWorkspaceMember {
        workspace_id: workspace.id,
        user_id: user.id,
        role_id: role.id,
    };

    // Test direct database insertion
    let created_member = create_workspace_member(&mut conn, new_member).await.unwrap();

    assert_eq!(created_member.workspace_id, workspace.id, "Workspace ID should match");
    assert_eq!(created_member.user_id, user.id, "User ID should match");
    assert_eq!(created_member.role_id, role.id, "Role ID should match");
}

#[tokio::test]
async fn test_create_workspace_member_complete_scenario() {
    let test_app = TestApp::new("test_create_workspace_member_complete_scenario").await;
    let mut conn = test_app.get_connection().await;

    // Use the complete test scenario helper
    let (user, workspace, role, member) = test_app.create_complete_test_scenario().await.unwrap();

    // Verify all relationships are correct
    assert_eq!(member.workspace_id, workspace.id, "Member should belong to workspace");
    assert_eq!(member.user_id, user.id, "Member should be the user");
    assert_eq!(member.role_id, role.id, "Member should have the role");

    // Verify member count
    let member_count = test_app.count_workspace_members(workspace.id).await.unwrap();
    assert_eq!(member_count, 1, "Should have exactly one member");

    // Verify membership
    let is_member = test_app.is_workspace_member(workspace.id, user.id).await.unwrap();
    assert!(is_member, "User should be a workspace member");
}