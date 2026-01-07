use buildscale::{
    models::roles::NewRole,
    queries::roles::create_role,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_create_role_query() {
    let test_app = TestApp::new("test_create_role_query").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first (required for role)
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();
    let workspace_id = workspace.id;

    // Create a NewRole manually (bypassing service layer)
    let new_role = NewRole {
        workspace_id,
        name: format!("{}_test_role", test_app.test_prefix()),
        description: Some("Test Role Description".to_string()),
    };

    // Test direct database insertion
    let created_role = create_role(&mut conn, new_role).await.unwrap();

    assert_eq!(created_role.workspace_id, workspace_id, "Workspace ID should match");
    assert_eq!(
        created_role.name,
        format!("{}_test_role", test_app.test_prefix()),
        "Role name should match"
    );
    assert_eq!(
        created_role.description,
        Some("Test Role Description".to_string()),
        "Role description should match"
    );
    assert!(
        !created_role.id.to_string().is_empty(),
        "Role ID should be populated"
    );
}

#[tokio::test]
async fn test_create_role_without_description() {
    let test_app = TestApp::new("test_create_role_without_description").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create a role without description
    let new_role = NewRole {
        workspace_id: workspace.id,
        name: format!("{}_role_no_desc", test_app.test_prefix()),
        description: None,
    };

    let created_role = create_role(&mut conn, new_role).await.unwrap();

    assert_eq!(created_role.workspace_id, workspace.id, "Workspace ID should match");
    assert_eq!(
        created_role.name,
        format!("{}_role_no_desc", test_app.test_prefix()),
        "Role name should match"
    );
    assert_eq!(
        created_role.description,
        None,
        "Description should be None"
    );
}

#[tokio::test]
async fn test_create_role_with_long_name() {
    let test_app = TestApp::new("test_create_role_with_long_name").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create a role with a long name
    let long_name = format!("{}_very_long_role_name_that_is_still_valid", test_app.test_prefix());
    let new_role = NewRole {
        workspace_id: workspace.id,
        name: long_name.clone(),
        description: Some("Role with long name".to_string()),
    };

    let created_role = create_role(&mut conn, new_role).await.unwrap();

    assert_eq!(created_role.name, long_name, "Long role name should be preserved");
}