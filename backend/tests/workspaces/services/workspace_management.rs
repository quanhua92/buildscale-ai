use backend::{
    services::workspaces::create_workspace,
    queries::workspaces::get_workspace_by_id,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_workspace_creation_success() {
    let test_app = TestApp::new("test_workspace_creation_success").await;
    let mut conn = test_app.get_connection().await;

    let initial_count = test_app.count_test_workspaces().await.unwrap();

    // Create a user first for the service layer test
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();

    // For service layer testing, we need to test the service directly
    let workspace_data = backend::models::workspaces::NewWorkspace {
        name: format!("{}_test_workspace", test_app.test_prefix()),
        owner_id: user.id,
    };
    let result = create_workspace(&mut conn, workspace_data).await;
    assert!(result.is_ok(), "Workspace creation should succeed");

    let created_workspace = result.unwrap();
    assert!(
        !created_workspace.id.to_string().is_empty(),
        "Workspace should have a valid UUID"
    );
    assert_eq!(created_workspace.owner_id, user.id, "Workspace owner should match");
    assert!(
        created_workspace.created_at <= chrono::Utc::now(),
        "Created timestamp should be valid"
    );
    assert!(
        created_workspace.updated_at <= chrono::Utc::now(),
        "Updated timestamp should be valid"
    );

    let final_count = test_app.count_test_workspaces().await.unwrap();
    assert_eq!(
        final_count,
        initial_count + 2,
        "Workspace count should increase by 2 (one from helper, one from service)"
    );

    // Verify workspace exists in database
    assert!(
        test_app.workspace_exists(&created_workspace.name).await.unwrap(),
        "Workspace should exist in database"
    );
}

#[tokio::test]
async fn test_workspace_creation_empty_name_validation() {
    let test_app = TestApp::new("test_workspace_creation_empty_name_validation").await;
    let mut conn = test_app.get_connection().await;

    // Test empty workspace name - create a user first to get a valid owner_id
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();
    let mut workspace_data = test_app.generate_test_workspace_with_owner_id(user.id);
    workspace_data.name = String::new();

    let result = create_workspace(&mut conn, workspace_data).await;
    assert!(
        result.is_err(),
        "Empty workspace name should cause creation to fail"
    );

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("empty"),
        "Error message should mention empty name: {}",
        error_message
    );
}

#[tokio::test]
async fn test_workspace_creation_whitespace_name_validation() {
    let test_app = TestApp::new("test_workspace_creation_whitespace_name_validation").await;
    let mut conn = test_app.get_connection().await;

    // Test whitespace-only workspace name - create a user first to get a valid owner_id
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();
    let mut workspace_data = test_app.generate_test_workspace_with_owner_id(user.id);
    workspace_data.name = "   ".to_string();

    let result = create_workspace(&mut conn, workspace_data).await;
    assert!(
        result.is_err(),
        "Whitespace-only workspace name should cause creation to fail"
    );

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("empty"),
        "Error message should mention empty name: {}",
        error_message
    );
}

#[tokio::test]
async fn test_workspace_creation_name_length_validation() {
    let test_app = TestApp::new("test_workspace_creation_name_length_validation").await;
    let mut conn = test_app.get_connection().await;

    // Test workspace name that's too long (> 100 characters) - create a user first to get a valid owner_id
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();
    let mut workspace_data = test_app.generate_test_workspace_with_owner_id(user.id);
    workspace_data.name = "a".repeat(101);

    let result = create_workspace(&mut conn, workspace_data).await;
    assert!(
        result.is_err(),
        "Workspace name longer than 100 characters should cause creation to fail"
    );

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("100") || error_message.contains("length"),
        "Error message should mention length requirement: {}",
        error_message
    );
}

#[tokio::test]
async fn test_workspace_creation_max_valid_name() {
    let test_app = TestApp::new("test_workspace_creation_max_valid_name").await;
    let mut conn = test_app.get_connection().await;

    // Test workspace name that's exactly 100 characters (should succeed) - create a user first to get a valid owner_id
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();
    let mut workspace_data = test_app.generate_test_workspace_with_owner_id(user.id);
    workspace_data.name = "a".repeat(100);

    let result = create_workspace(&mut conn, workspace_data).await;
    assert!(result.is_ok(), "100-character workspace name should be valid");
}

#[tokio::test]
async fn test_workspace_deletion_service() {
    let test_app = TestApp::new("test_workspace_deletion_service").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace with real user first
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create another workspace through the service for testing
    let workspace_data = backend::models::workspaces::NewWorkspace {
        name: format!("{}_delete_workspace", test_app.test_prefix()),
        owner_id: user.id,
    };
    let created_workspace = backend::services::workspaces::create_workspace(&mut conn, workspace_data).await.unwrap();

    // Verify workspace exists
    assert!(
        test_app.workspace_exists(&created_workspace.name).await.unwrap(),
        "Workspace should exist before deletion"
    );

    // Delete the workspace through service
    let result = backend::services::workspaces::delete_workspace(&mut conn, created_workspace.id).await;
    assert!(result.is_ok(), "Workspace deletion should succeed");

    // Verify workspace no longer exists
    let check_result = get_workspace_by_id(&mut conn, created_workspace.id).await;
    assert!(check_result.is_err(), "Workspace should not exist after deletion");
}

#[tokio::test]
async fn test_workspace_deletion_nonexistent() {
    let test_app = TestApp::new("test_workspace_deletion_nonexistent").await;
    let mut conn = test_app.get_connection().await;

    // Test deleting non-existent workspace
    let fake_id = uuid::Uuid::now_v7();
    let result = backend::services::workspaces::delete_workspace(&mut conn, fake_id).await;
    assert!(
        result.is_err(),
        "Deleting non-existent workspace should fail"
    );

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("not found"),
        "Error message should mention not found: {}",
        error_message
    );
}