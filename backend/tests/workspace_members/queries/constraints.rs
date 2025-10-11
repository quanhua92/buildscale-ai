use backend::{
    models::workspace_members::NewWorkspaceMember,
    queries::workspace_members::create_workspace_member,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_workspace_member_unique_constraint() {
    let test_app = TestApp::new("test_workspace_member_unique_constraint").await;
    let mut conn = test_app.get_connection().await;

    // Create a complete test scenario
    let (user, workspace, role, _) = test_app.create_complete_test_scenario().await.unwrap();

    // Try to create the same membership again (should fail)
    let duplicate_member = NewWorkspaceMember {
        workspace_id: workspace.id,
        user_id: user.id,
        role_id: role.id,
    };

    let result = create_workspace_member(&mut conn, duplicate_member).await;
    assert!(
        result.is_err(),
        "Duplicate workspace membership should violate unique constraint"
    );
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("duplicate") || error_message.contains("unique"),
        "Error should be related to database constraint: {}",
        error_message
    );
}

#[tokio::test]
async fn test_workspace_member_foreign_key_constraints() {
    let test_app = TestApp::new("test_workspace_member_foreign_key_constraints").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace and role first with real user
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let role_data = test_app.generate_test_role(workspace.id);
    let role = backend::queries::roles::create_role(&mut conn, role_data).await.unwrap();

    // Test creating member with non-existent user ID
    let fake_user_id = uuid::Uuid::now_v7();
    let invalid_member = NewWorkspaceMember {
        workspace_id: workspace.id,
        user_id: fake_user_id,
        role_id: role.id,
    };

    let result = create_workspace_member(&mut conn, invalid_member).await;
    assert!(
        result.is_err(),
        "Member with non-existent user should violate foreign key constraint"
    );
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("foreign") || error_message.contains("constraint"),
        "Error should be related to foreign key constraint: {}",
        error_message
    );
}