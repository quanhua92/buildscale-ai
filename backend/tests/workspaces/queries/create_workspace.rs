use backend::{
    models::workspaces::NewWorkspace,
    queries::workspaces::create_workspace,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_create_workspace_query() {
    let test_app = TestApp::new("test_create_workspace_query").await;
    let mut conn = test_app.get_connection().await;

    // Create a NewWorkspace manually (bypassing service layer)
    let owner_id = test_app.generate_test_uuid();
    let new_workspace = NewWorkspace {
        name: format!("{}_test_workspace", test_app.test_prefix()),
        owner_id,
    };

    // Test direct database insertion
    let created_workspace = create_workspace(&mut conn, new_workspace).await.unwrap();

    assert_eq!(
        created_workspace.name,
        format!("{}_test_workspace", test_app.test_prefix()),
        "Workspace name should match"
    );
    assert_eq!(created_workspace.owner_id, owner_id, "Owner ID should match");
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
    let mut conn = test_app.get_connection().await;

    let owner_id = test_app.generate_test_uuid();
    let new_workspace = NewWorkspace {
        name: format!("{}_owned_workspace", test_app.test_prefix()),
        owner_id,
    };

    let created_workspace = create_workspace(&mut conn, new_workspace).await.unwrap();

    assert_eq!(created_workspace.owner_id, owner_id, "Owner ID should match");
}

#[tokio::test]
async fn test_create_workspace_long_name() {
    let test_app = TestApp::new("test_create_workspace_long_name").await;
    let mut conn = test_app.get_connection().await;

    let long_name = format!("{}_very_long_workspace_name_that_is_still_valid", test_app.test_prefix());
    let new_workspace = NewWorkspace {
        name: long_name.clone(),
        owner_id: test_app.generate_test_uuid(),
    };

    let created_workspace = create_workspace(&mut conn, new_workspace).await.unwrap();

    assert_eq!(created_workspace.name, long_name, "Long workspace name should be preserved");
}