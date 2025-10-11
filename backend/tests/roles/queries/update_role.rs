use backend::{
    models::roles::UpdateRole,
    queries::roles::{create_role, update_role},
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_update_role_query() {
    let test_app = TestApp::new("test_update_role_query").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace and role first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let new_role = test_app.generate_test_role(workspace.id);
    let created_role = create_role(&mut conn, new_role).await.unwrap();
    let role_id = created_role.id;

    // Update the role
    let update_data = UpdateRole {
        name: Some(format!("{}_updated_role", test_app.test_prefix())),
        description: Some("Updated role description".to_string()),
    };

    let updated_role = update_role(&mut conn, role_id, update_data).await.unwrap();

    assert_eq!(updated_role.id, role_id, "Role ID should not change");
    assert_eq!(updated_role.workspace_id, workspace.id, "Workspace ID should not change");
    assert_eq!(
        updated_role.name,
        format!("{}_updated_role", test_app.test_prefix()),
        "Role name should be updated"
    );
    assert_eq!(
        updated_role.description,
        Some("Updated role description".to_string()),
        "Description should be updated"
    );
}

#[tokio::test]
async fn test_update_role_partial_update() {
    let test_app = TestApp::new("test_update_role_partial_update").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace and role first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let new_role = test_app.generate_test_role(workspace.id);
    let created_role = create_role(&mut conn, new_role).await.unwrap();
    let role_id = created_role.id;

    // Update only the name (partial update)
    let update_data = UpdateRole {
        name: Some(format!("{}_partial_update", test_app.test_prefix())),
        description: None, // Keep original description
    };

    let updated_role = update_role(&mut conn, role_id, update_data).await.unwrap();

    assert_eq!(
        updated_role.name,
        format!("{}_partial_update", test_app.test_prefix()),
        "Name should be updated"
    );
    assert_eq!(
        updated_role.description,
        Some("Test role description".to_string()),
        "Original description should be preserved"
    );
}

#[tokio::test]
async fn test_update_role_no_changes() {
    let test_app = TestApp::new("test_update_role_no_changes").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace and role first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let new_role = test_app.generate_test_role(workspace.id);
    let created_role = create_role(&mut conn, new_role).await.unwrap();
    let role_id = created_role.id;

    // Update with no changes
    let update_data = UpdateRole {
        name: None,
        description: None,
    };

    let updated_role = update_role(&mut conn, role_id, update_data).await.unwrap();

    assert_eq!(updated_role.name, created_role.name, "Name should not change");
    assert_eq!(updated_role.description, created_role.description, "Description should not change");
}

#[tokio::test]
async fn test_update_role_clear_description() {
    let test_app = TestApp::new("test_update_role_clear_description").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace and role first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let new_role = test_app.generate_test_role(workspace.id);
    let created_role = create_role(&mut conn, new_role).await.unwrap();
    let role_id = created_role.id;

    // Clear the description
    let update_data = UpdateRole {
        name: None,
        description: Some(String::new()), // Empty string should become None
    };

    let updated_role = update_role(&mut conn, role_id, update_data).await.unwrap();

    assert_eq!(updated_role.name, created_role.name, "Name should not change");
    assert_eq!(updated_role.description, Some(String::new()), "Description should be empty string");
}