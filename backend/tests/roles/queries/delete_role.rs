use buildscale::{
    queries::roles::{create_role, delete_role, get_role_by_id},
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_delete_role_query() {
    let test_app = TestApp::new("test_delete_role_query").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace and role first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let new_role = test_app.generate_test_role(workspace.id);
    let created_role = create_role(&mut conn, new_role).await.unwrap();
    let role_id = created_role.id;

    // Verify role exists before deletion
    let found_role = get_role_by_id(&mut conn, role_id).await.unwrap();
    assert_eq!(found_role.id, role_id, "Role should exist before deletion");

    // Delete the role
    let rows_affected = delete_role(&mut conn, role_id).await.unwrap();
    assert_eq!(rows_affected, 1, "Should delete exactly one role");

    // Verify role no longer exists
    let result = get_role_by_id(&mut conn, role_id).await;
    assert!(result.is_err(), "Role should no longer exist after deletion");
}

#[tokio::test]
async fn test_delete_nonexistent_role() {
    let test_app = TestApp::new("test_delete_nonexistent_role").await;
    let mut conn = test_app.get_connection().await;

    // Test with non-existent UUID
    let fake_id = uuid::Uuid::now_v7();
    let rows_affected = delete_role(&mut conn, fake_id).await.unwrap();

    assert_eq!(rows_affected, 0, "Should not delete any rows for non-existent role");
}

#[tokio::test]
async fn test_delete_role_multiple_roles() {
    let test_app = TestApp::new("test_delete_role_multiple_roles").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace with real user
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create multiple roles
    let role1_data = test_app.generate_test_role_with_name(workspace.id, "role1");
    let role2_data = test_app.generate_test_role_with_name(workspace.id, "role2");
    let role3_data = test_app.generate_test_role_with_name(workspace.id, "role3");

    let role1 = create_role(&mut conn, role1_data).await.unwrap();
    let role2 = create_role(&mut conn, role2_data).await.unwrap();
    let role3 = create_role(&mut conn, role3_data).await.unwrap();

    // Delete the middle role
    let rows_affected = delete_role(&mut conn, role2.id).await.unwrap();
    assert_eq!(rows_affected, 1, "Should delete exactly one role");

    // Verify other roles still exist
    let found_role1 = get_role_by_id(&mut conn, role1.id).await.unwrap();
    assert_eq!(found_role1.id, role1.id, "Role 1 should still exist");

    let found_role3 = get_role_by_id(&mut conn, role3.id).await.unwrap();
    assert_eq!(found_role3.id, role3.id, "Role 3 should still exist");

    // Verify deleted role no longer exists
    let result = get_role_by_id(&mut conn, role2.id).await;
    assert!(result.is_err(), "Deleted role should no longer exist");
}