use backend::{
    queries::roles::{create_role, list_roles, list_roles_by_workspace},
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_list_roles_empty() {
    let test_app = TestApp::new("test_list_roles_empty").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // List roles for workspace (should be empty)
    let roles = list_roles_by_workspace(&mut conn, workspace.id).await.unwrap();

    assert_eq!(roles.len(), 0, "Should return empty list for workspace with no roles");
}

#[tokio::test]
async fn test_list_roles_by_workspace() {
    let test_app = TestApp::new("test_list_roles_by_workspace").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create multiple roles
    let role_names = vec!["admin", "editor", "viewer"];
    let mut created_roles = Vec::new();

    for role_name in role_names {
        let new_role = test_app.generate_test_role_with_name(workspace.id, role_name);
        let created_role = create_role(&mut conn, new_role).await.unwrap();
        created_roles.push(created_role);
    }

    // List roles for workspace
    let roles = list_roles_by_workspace(&mut conn, workspace.id).await.unwrap();

    assert_eq!(roles.len(), 3, "Should return all roles for workspace");

    // Verify roles are sorted by name
    let mut sorted_roles = created_roles.clone();
    sorted_roles.sort_by(|a, b| a.name.cmp(&b.name));

    for (i, role) in roles.iter().enumerate() {
        assert_eq!(role.id, sorted_roles[i].id, "Role ID should match");
        assert_eq!(role.name, sorted_roles[i].name, "Role name should match");
        assert_eq!(role.workspace_id, workspace.id, "Workspace ID should match");
    }
}

#[tokio::test]
async fn test_list_roles_by_workspace_filtered() {
    let test_app = TestApp::new("test_list_roles_by_workspace_filtered").await;
    let mut conn = test_app.get_connection().await;

    // Create two workspaces with real users
    let (_, workspace1) = test_app.create_test_workspace_with_user().await.unwrap();
    let (_, workspace2) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create roles in both workspaces
    let role1_data = test_app.generate_test_role_with_name(workspace1.id, "role1");
    let role2_data = test_app.generate_test_role_with_name(workspace1.id, "role2");
    let role3_data = test_app.generate_test_role_with_name(workspace2.id, "role3");

    create_role(&mut conn, role1_data).await.unwrap();
    create_role(&mut conn, role2_data).await.unwrap();
    create_role(&mut conn, role3_data).await.unwrap();

    // List roles for workspace1 only
    let workspace1_roles = list_roles_by_workspace(&mut conn, workspace1.id).await.unwrap();
    assert_eq!(workspace1_roles.len(), 2, "Should return only workspace1 roles");

    // List roles for workspace2 only
    let workspace2_roles = list_roles_by_workspace(&mut conn, workspace2.id).await.unwrap();
    assert_eq!(workspace2_roles.len(), 1, "Should return only workspace2 roles");

    // Verify all roles belong to correct workspace
    for role in &workspace1_roles {
        assert_eq!(role.workspace_id, workspace1.id, "All roles should belong to workspace1");
    }

    for role in &workspace2_roles {
        assert_eq!(role.workspace_id, workspace2.id, "All roles should belong to workspace2");
    }
}

#[tokio::test]
async fn test_list_roles_all() {
    let test_app = TestApp::new("test_list_roles_all").await;
    let mut conn = test_app.get_connection().await;

    // Create multiple workspaces and roles with real users
    let (_, workspace1) = test_app.create_test_workspace_with_user().await.unwrap();
    let (_, workspace2) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create roles in both workspaces using custom names that include test prefix
    let role1_data = backend::models::roles::NewRole {
        workspace_id: workspace1.id,
        name: format!("{}_admin", test_app.test_prefix()),
        description: Some("Test role description".to_string()),
    };
    let role2_data = backend::models::roles::NewRole {
        workspace_id: workspace1.id,
        name: format!("{}_editor", test_app.test_prefix()),
        description: Some("Test role description".to_string()),
    };
    let role3_data = backend::models::roles::NewRole {
        workspace_id: workspace2.id,
        name: format!("{}_viewer", test_app.test_prefix()),
        description: Some("Test role description".to_string()),
    };

    let role1 = create_role(&mut conn, role1_data).await.unwrap();
    let role2 = create_role(&mut conn, role2_data).await.unwrap();
    let role3 = create_role(&mut conn, role3_data).await.unwrap();

    // List all roles and filter by test prefix
    let all_roles = list_roles(&mut conn).await.unwrap();
    let test_roles: Vec<_> = all_roles.iter()
        .filter(|r| r.name.starts_with(&test_app.test_prefix()))
        .collect();

    assert_eq!(test_roles.len(), 3, "Should return all test roles from all workspaces");

    // Verify all created roles are in the list
    let role_ids: Vec<_> = test_roles.iter().map(|r| r.id).collect();
    assert!(role_ids.contains(&role1.id), "Role1 should be in the list");
    assert!(role_ids.contains(&role2.id), "Role2 should be in the list");
    assert!(role_ids.contains(&role3.id), "Role3 should be in the list");
}

#[tokio::test]
async fn test_list_roles_ordering() {
    let test_app = TestApp::new("test_list_roles_ordering").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace first
    let (_, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create roles in non-alphabetical order
    let role_names = vec!["zebra", "alpha", "beta"];
    let mut created_roles = Vec::new();

    for role_name in role_names {
        let new_role = test_app.generate_test_role_with_name(workspace.id, role_name);
        let created_role = create_role(&mut conn, new_role).await.unwrap();
        created_roles.push(created_role);
    }

    // List roles for workspace
    let roles = list_roles_by_workspace(&mut conn, workspace.id).await.unwrap();

    assert_eq!(roles.len(), 3, "Should return all roles");

    // Verify roles are sorted alphabetically by name
    assert_eq!(roles[0].name, "alpha", "First role should be 'alpha'");
    assert_eq!(roles[1].name, "beta", "Second role should be 'beta'");
    assert_eq!(roles[2].name, "zebra", "Third role should be 'zebra'");
}