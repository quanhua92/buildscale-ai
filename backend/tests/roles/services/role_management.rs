use backend::{
    services::roles::create_role,
    queries::roles::get_role_by_id,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_role_creation_success() {
    let test_app = TestApp::new("test_role_creation_success").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let initial_count = test_app.count_test_roles().await.unwrap();

    // Create a role through the service layer
    let role_data = test_app.generate_test_role(workspace.id);
    let role_name = role_data.name.clone();

    let result = create_role(&mut conn, role_data).await;
    assert!(result.is_ok(), "Role creation should succeed");

    let created_role = result.unwrap();
    assert!(
        !created_role.id.to_string().is_empty(),
        "Role should have a valid UUID"
    );
    assert_eq!(created_role.name, role_name, "Role name should match");
    assert_eq!(created_role.workspace_id, workspace.id, "Workspace ID should match");
    assert!(
        created_role.description.is_some(),
        "Description should be populated"
    );

    let final_count = test_app.count_test_roles().await.unwrap();
    assert_eq!(
        final_count,
        initial_count + 1,
        "Role count should increase by 1"
    );

    // Verify role exists in database
    assert!(
        test_app.role_exists(workspace.id, &created_role.name).await.unwrap(),
        "Role should exist in database"
    );
}

#[tokio::test]
async fn test_role_creation_empty_name_validation() {
    let test_app = TestApp::new("test_role_creation_empty_name_validation").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test empty role name
    let mut role_data = test_app.generate_test_role(workspace.id);
    role_data.name = String::new();

    let result = create_role(&mut conn, role_data).await;
    assert!(
        result.is_err(),
        "Empty role name should cause creation to fail"
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
async fn test_role_creation_whitespace_name_validation() {
    let test_app = TestApp::new("test_role_creation_whitespace_name_validation").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test whitespace-only role name
    let mut role_data = test_app.generate_test_role(workspace.id);
    role_data.name = "   ".to_string();

    let result = create_role(&mut conn, role_data).await;
    assert!(
        result.is_err(),
        "Whitespace-only role name should cause creation to fail"
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
async fn test_role_creation_name_length_validation() {
    let test_app = TestApp::new("test_role_creation_name_length_validation").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test role name that's too long (> 100 characters)
    let mut role_data = test_app.generate_test_role(workspace.id);
    role_data.name = "a".repeat(101);

    let result = create_role(&mut conn, role_data).await;
    assert!(
        result.is_err(),
        "Role name longer than 100 characters should cause creation to fail"
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
async fn test_role_creation_max_valid_name() {
    let test_app = TestApp::new("test_role_creation_max_valid_name").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test role name that's exactly 100 characters (should succeed)
    let mut role_data = test_app.generate_test_role(workspace.id);
    role_data.name = "a".repeat(100);

    let result = create_role(&mut conn, role_data).await;
    assert!(result.is_ok(), "100-character role name should be valid");
}

#[tokio::test]
async fn test_role_creation_duplicate_name_validation() {
    let test_app = TestApp::new("test_role_creation_duplicate_name_validation").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let role_name = format!("{}_duplicate_test", test_app.test_prefix());

    // Create first role
    let role_data1 = test_app.generate_test_role_with_name(workspace.id, &role_name);
    create_role(&mut conn, role_data1).await.unwrap();

    // Try to create second role with same name
    let role_data2 = test_app.generate_test_role_with_name(workspace.id, &role_name);
    let result = create_role(&mut conn, role_data2).await;
    assert!(
        result.is_err(),
        "Duplicate role name should cause creation to fail"
    );

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("already exists"),
        "Error message should mention duplicate: {}",
        error_message
    );
}

#[tokio::test]
async fn test_role_creation_description_length_validation() {
    let test_app = TestApp::new("test_role_creation_description_length_validation").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test role description that's too long (> 500 characters)
    let mut role_data = test_app.generate_test_role(workspace.id);
    role_data.description = Some("a".repeat(501));

    let result = create_role(&mut conn, role_data).await;
    assert!(
        result.is_err(),
        "Role description longer than 500 characters should cause creation to fail"
    );

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("500") || error_message.contains("length"),
        "Error message should mention description length requirement: {}",
        error_message
    );
}

#[tokio::test]
async fn test_role_creation_max_valid_description() {
    let test_app = TestApp::new("test_role_creation_max_valid_description").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test role description that's exactly 500 characters (should succeed)
    let mut role_data = test_app.generate_test_role(workspace.id);
    role_data.description = Some("a".repeat(500));

    let result = create_role(&mut conn, role_data).await;
    assert!(result.is_ok(), "500-character role description should be valid");
}

#[tokio::test]
async fn test_role_deletion_service() {
    let test_app = TestApp::new("test_role_deletion_service").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create a role
    let role_data = test_app.generate_test_role(workspace.id);
    let created_role = backend::services::roles::create_role(&mut conn, role_data).await.unwrap();

    // Verify role exists
    assert!(
        test_app.role_exists(workspace.id, &created_role.name).await.unwrap(),
        "Role should exist before deletion"
    );

    // Delete the role through service
    let result = backend::services::roles::delete_role(&mut conn, created_role.id).await;
    assert!(result.is_ok(), "Role deletion should succeed");

    // Verify role no longer exists
    let check_result = get_role_by_id(&mut conn, created_role.id).await;
    assert!(check_result.is_err(), "Role should not exist after deletion");
}

#[tokio::test]
async fn test_role_deletion_nonexistent() {
    let test_app = TestApp::new("test_role_deletion_nonexistent").await;
    let mut conn = test_app.get_connection().await;

    // Test deleting non-existent role
    let fake_id = uuid::Uuid::now_v7();
    let result = backend::services::roles::delete_role(&mut conn, fake_id).await;
    assert!(
        result.is_err(),
        "Deleting non-existent role should fail"
    );

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("not found"),
        "Error message should mention not found: {}",
        error_message
    );
}