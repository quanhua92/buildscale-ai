use backend::{
    models::roles::NewRole,
    queries::roles::{create_role, get_role_by_id},
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_role_field_constraints() {
    let test_app = TestApp::new("test_role_field_constraints").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test that role name constraint works (unique constraint within workspace)
    let role_name = format!("{}_unique_role", test_app.test_prefix());
    let new_role1 = NewRole {
        workspace_id: workspace.id,
        name: role_name.clone(),
        description: Some("First role".to_string()),
    };

    let new_role2 = NewRole {
        workspace_id: workspace.id,
        name: role_name.clone(),
        description: Some("Second role".to_string()),
    };

    // First role should succeed
    create_role(&mut conn, new_role1).await.unwrap();

    // Second role with same name in same workspace should fail
    let result = create_role(&mut conn, new_role2).await;
    assert!(
        result.is_err(),
        "Duplicate role name should violate unique constraint"
    );
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("duplicate") || error_message.contains("unique"),
        "Error should be related to database constraint: {}",
        error_message
    );
}

#[tokio::test]
async fn test_role_name_unique_per_workspace() {
    let test_app = TestApp::new("test_role_name_unique_per_workspace").await;
    let mut conn = test_app.get_connection().await;

    // Create two workspaces
    let (_, workspace1) = test_app.create_test_workspace_with_user().await.unwrap();
    let second_app = TestApp::new(&format!("{}_second", test_app.test_prefix())).await;
    let (_, workspace2) = second_app.create_test_workspace_with_user().await.unwrap();

    // Create roles with same name in different workspaces (should succeed)
    let role_name = format!("{}_shared_role_name", test_app.test_prefix());
    let new_role1 = NewRole {
        workspace_id: workspace1.id,
        name: role_name.clone(),
        description: Some("Role in workspace1".to_string()),
    };

    let new_role2 = NewRole {
        workspace_id: workspace2.id,
        name: role_name.clone(),
        description: Some("Role in workspace2".to_string()),
    };

    // Both roles should succeed (names are unique per workspace)
    let role1 = create_role(&mut conn, new_role1).await.unwrap();
    let role2 = create_role(&mut conn, new_role2).await.unwrap();

    assert_ne!(role1.id, role2.id, "Roles should have different IDs");
    assert_eq!(role1.name, role_name, "Role1 name should match");
    assert_eq!(role2.name, role_name, "Role2 name should match");
    assert_eq!(role1.workspace_id, workspace1.id, "Role1 should belong to workspace1");
    assert_eq!(role2.workspace_id, workspace2.id, "Role2 should belong to workspace2");
}

#[tokio::test]
async fn test_role_foreign_key_constraint() {
    let test_app = TestApp::new("test_role_foreign_key_constraint").await;
    let mut conn = test_app.get_connection().await;

    // Test creating role with non-existent workspace ID
    let fake_workspace_id = uuid::Uuid::now_v7();
    let new_role = NewRole {
        workspace_id: fake_workspace_id,
        name: format!("{}_orphan_role", test_app.test_prefix()),
        description: Some("Orphan role".to_string()),
    };

    let result = create_role(&mut conn, new_role).await;
    assert!(
        result.is_err(),
        "Role with non-existent workspace should violate foreign key constraint"
    );
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("foreign") || error_message.contains("constraint") || error_message.contains("violates"),
        "Error should be related to foreign key constraint: {}",
        error_message
    );
}

#[tokio::test]
async fn test_query_error_handling() {
    let test_app = TestApp::new("test_query_error_handling").await;
    let mut conn = test_app.get_connection().await;

    // Test getting non-existent role by ID
    let fake_id = uuid::Uuid::now_v7();
    let result = get_role_by_id(&mut conn, fake_id).await;

    assert!(result.is_err(), "Non-existent role should cause error");
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("no rows") || error_message.contains("found"),
        "Error should indicate role not found: {}",
        error_message
    );
}