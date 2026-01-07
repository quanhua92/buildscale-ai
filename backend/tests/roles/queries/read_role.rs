use buildscale::{
    queries::roles::{create_role, get_role_by_id, get_role_by_workspace_and_name},
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_get_role_by_id_query() {
    let test_app = TestApp::new("test_get_role_by_id_query").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create a role
    let new_role = test_app.generate_test_role(workspace.id);
    let created_role = create_role(&mut conn, new_role).await.unwrap();
    let role_id = created_role.id;

    // Test getting role by ID
    let found_role = get_role_by_id(&mut conn, role_id).await.unwrap();

    assert_eq!(found_role.id, role_id, "Role ID should match");
    assert_eq!(found_role.workspace_id, workspace.id, "Workspace ID should match");
    assert_eq!(
        found_role.name,
        format!("{}_role", test_app.test_prefix()),
        "Role name should match"
    );
    assert_eq!(
        found_role.description,
        Some("Test role description".to_string()),
        "Description should match"
    );
}

#[tokio::test]
async fn test_get_role_by_id_not_found() {
    let test_app = TestApp::new("test_get_role_by_id_not_found").await;
    let mut conn = test_app.get_connection().await;

    // Test with non-existent UUID
    let fake_id = uuid::Uuid::now_v7();
    let result = get_role_by_id(&mut conn, fake_id).await;

    assert!(result.is_err(), "Should return error for non-existent role");
    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("no rows") || error_message.contains("found"),
        "Error should indicate role not found: {}",
        error_message
    );
}

#[tokio::test]
async fn test_get_role_by_workspace_and_name() {
    let test_app = TestApp::new("test_get_role_by_workspace_and_name").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create a role
    let role_name = format!("{}_test_role", test_app.test_prefix());
    let new_role = test_app.generate_test_role_with_name(workspace.id, &role_name);
    let created_role = create_role(&mut conn, new_role).await.unwrap();

    // Test getting role by workspace and name
    let found_role = get_role_by_workspace_and_name(&mut conn, workspace.id, &role_name).await.unwrap();

    assert!(found_role.is_some(), "Role should be found");
    let found_role = found_role.unwrap();

    assert_eq!(found_role.id, created_role.id, "Role ID should match");
    assert_eq!(found_role.workspace_id, workspace.id, "Workspace ID should match");
    assert_eq!(found_role.name, role_name, "Role name should match");
}

#[tokio::test]
async fn test_get_role_by_workspace_and_name_not_found() {
    let test_app = TestApp::new("test_get_role_by_workspace_and_name_not_found").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test with non-existent role name
    let fake_role_name = "non_existent_role";
    let found_role = get_role_by_workspace_and_name(&mut conn, workspace.id, fake_role_name).await.unwrap();

    assert!(found_role.is_none(), "Non-existent role should return None");

    // Test with non-existent workspace ID
    let fake_workspace_id = uuid::Uuid::now_v7();
    let found_role = get_role_by_workspace_and_name(&mut conn, fake_workspace_id, "some_role").await.unwrap();

    assert!(found_role.is_none(), "Non-existent workspace should return None");
}