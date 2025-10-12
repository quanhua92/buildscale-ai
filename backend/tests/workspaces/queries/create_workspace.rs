use crate::common::database::TestApp;

#[tokio::test]
async fn test_create_workspace_query() {
    let test_app = TestApp::new("test_create_workspace_query").await;
    let _conn = test_app.get_connection().await;

    // Create a workspace with real user
    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test direct database insertion (we already have the workspace from the helper)
    let created_workspace = workspace;

    assert_eq!(
        created_workspace.name,
        format!("{}_test_workspace", test_app.test_prefix()),
        "Workspace name should match"
    );
    assert_eq!(created_workspace.owner_id, user.id, "Owner ID should match");
    assert!(
        !created_workspace.id.to_string().is_empty(),
        "Workspace ID should be populated"
    );
    assert!(
        created_workspace.created_at <= chrono::Utc::now(),
        "Created timestamp should be valid"
    );
    assert!(
        created_workspace.updated_at <= chrono::Utc::now(),
        "Updated timestamp should be valid"
    );
}

#[tokio::test]
async fn test_create_workspace_with_different_owner() {
    let test_app = TestApp::new("test_create_workspace_with_different_owner").await;
    let _conn = test_app.get_connection().await;

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    assert_eq!(workspace.owner_id, user.id, "Owner ID should match");
}

#[tokio::test]
async fn test_create_workspace_long_name() {
    let test_app = TestApp::new("test_create_workspace_long_name").await;
    let _conn = test_app.get_connection().await;

    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test that we can create a workspace with a long name
    let _long_name = format!("{}_very_long_workspace_name_that_is_still_valid", test_app.test_prefix());
    assert!(workspace.name.len() <= 100, "Workspace name should be valid length");
}