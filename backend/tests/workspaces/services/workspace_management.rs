use backend::{
    services::workspaces::{create_workspace, create_workspace_with_members, delete_workspace, get_workspace},
    models::requests::{CreateWorkspaceRequest, CreateWorkspaceWithMembersRequest, WorkspaceMemberRequest},
    models::roles::{ADMIN_ROLE, EDITOR_ROLE, MEMBER_ROLE, VIEWER_ROLE},
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_comprehensive_workspace_creation_success() {
    let test_app = TestApp::new("test_comprehensive_workspace_creation_success").await;
    let mut conn = test_app.get_connection().await;

    let initial_count = test_app.count_test_workspaces().await.unwrap();

    // Create a user first for the service layer test
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();

    // Test the new comprehensive workspace creation method
    let workspace_request = CreateWorkspaceRequest {
        name: format!("{}_test_workspace", test_app.test_prefix()),
        owner_id: user.id,
    };
    let result = create_workspace(&mut conn, workspace_request).await;
    assert!(result.is_ok(), "Comprehensive workspace creation should succeed");

    let workspace_result = result.unwrap();
    assert!(
        !workspace_result.workspace.id.to_string().is_empty(),
        "Workspace should have a valid UUID"
    );
    assert_eq!(workspace_result.workspace.owner_id, user.id, "Workspace owner should match");
    assert!(
        workspace_result.workspace.created_at <= chrono::Utc::now(),
        "Created timestamp should be valid"
    );
    assert!(
        workspace_result.workspace.updated_at <= chrono::Utc::now(),
        "Updated timestamp should be valid"
    );

    // Verify default roles were created
    assert_eq!(workspace_result.roles.len(), 4, "Should create 4 default roles");
    let role_names: Vec<String> = workspace_result.roles.iter().map(|r| r.name.clone()).collect();
    assert!(role_names.contains(&ADMIN_ROLE.to_string()), "Should have admin role");
    assert!(role_names.contains(&EDITOR_ROLE.to_string()), "Should have editor role");
    assert!(role_names.contains(&MEMBER_ROLE.to_string()), "Should have member role");
    assert!(role_names.contains(&VIEWER_ROLE.to_string()), "Should have viewer role");

    // Verify owner was added as admin member
    assert_eq!(workspace_result.owner_membership.user_id, user.id, "Owner should be added as member");
    assert_eq!(workspace_result.members.len(), 1, "Should have 1 member (the owner)");

    let final_count = test_app.count_test_workspaces().await.unwrap();
    assert_eq!(
        final_count,
        initial_count + 2, // One from helper, one from comprehensive create
        "Workspace count should increase by 2"
    );

    // Verify workspace exists in database
    assert!(
        test_app.workspace_exists(&workspace_result.workspace.name).await.unwrap(),
        "Workspace should exist in database"
    );
}

#[tokio::test]
async fn test_comprehensive_workspace_creation_empty_name_validation() {
    let test_app = TestApp::new("test_comprehensive_workspace_creation_empty_name_validation").await;
    let mut conn = test_app.get_connection().await;

    // Create a user first to get a valid owner_id
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();

    let workspace_request = CreateWorkspaceRequest {
        name: String::new(),
        owner_id: user.id,
    };

    let result = create_workspace(&mut conn, workspace_request).await;
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
async fn test_comprehensive_workspace_creation_whitespace_name_validation() {
    let test_app = TestApp::new("test_comprehensive_workspace_creation_whitespace_name_validation").await;
    let mut conn = test_app.get_connection().await;

    // Create a user first to get a valid owner_id
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();

    let workspace_request = CreateWorkspaceRequest {
        name: "   ".to_string(),
        owner_id: user.id,
    };

    let result = create_workspace(&mut conn, workspace_request).await;
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
async fn test_comprehensive_workspace_creation_name_length_validation() {
    let test_app = TestApp::new("test_comprehensive_workspace_creation_name_length_validation").await;
    let mut conn = test_app.get_connection().await;

    // Create a user first to get a valid owner_id
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();

    let workspace_request = CreateWorkspaceRequest {
        name: "a".repeat(101),
        owner_id: user.id,
    };

    let result = create_workspace(&mut conn, workspace_request).await;
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
async fn test_comprehensive_workspace_creation_max_valid_name() {
    let test_app = TestApp::new("test_comprehensive_workspace_creation_max_valid_name").await;
    let mut conn = test_app.get_connection().await;

    // Create a user first to get a valid owner_id
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();

    let workspace_request = CreateWorkspaceRequest {
        name: "a".repeat(100),
        owner_id: user.id,
    };

    let result = create_workspace(&mut conn, workspace_request).await;
    assert!(result.is_ok(), "100-character workspace name should be valid");
}

#[tokio::test]
async fn test_workspace_creation_with_members() {
    let test_app = TestApp::new("test_workspace_creation_with_members").await;
    let mut conn = test_app.get_connection().await;

    // Create users for the test
    let (owner_user, _) = test_app.create_test_workspace_with_user().await.unwrap();
    let member1_data = test_app.generate_test_user();
    let member1_user = backend::services::users::register_user(&mut conn, member1_data).await.unwrap();
    let member2_data = test_app.generate_test_user();
    let member2_user = backend::services::users::register_user(&mut conn, member2_data).await.unwrap();

    let workspace_request = CreateWorkspaceWithMembersRequest {
        name: format!("{}_workspace_with_members", test_app.test_prefix()),
        owner_id: owner_user.id,
        members: vec![
            WorkspaceMemberRequest {
                user_id: member1_user.id,
                role_name: EDITOR_ROLE.to_string(),
            },
            WorkspaceMemberRequest {
                user_id: member2_user.id,
                role_name: VIEWER_ROLE.to_string(),
            },
        ],
    };

    let result = create_workspace_with_members(&mut conn, workspace_request).await;
    assert!(result.is_ok(), "Workspace creation with members should succeed");

    let workspace_result = result.unwrap();

    // Verify workspace was created
    assert_eq!(workspace_result.workspace.owner_id, owner_user.id, "Workspace owner should match");

    // Verify default roles were created
    assert_eq!(workspace_result.roles.len(), 4, "Should create 4 default roles");

    // Verify all members were added (owner + 2 additional members)
    assert_eq!(workspace_result.members.len(), 3, "Should have 3 members total");

    // Verify owner was added as admin
    let owner_member = workspace_result.members.iter()
        .find(|m| m.user_id == owner_user.id)
        .expect("Owner should be in members list");

    // Find admin role ID
    let admin_role = workspace_result.roles.iter()
        .find(|r| r.name == ADMIN_ROLE)
        .expect("Should have admin role");

    assert_eq!(owner_member.role_id, admin_role.id, "Owner should have admin role");

    // Verify other members have correct roles
    let member1 = workspace_result.members.iter()
        .find(|m| m.user_id == member1_user.id)
        .expect("Member1 should be in members list");

    let editor_role = workspace_result.roles.iter()
        .find(|r| r.name == EDITOR_ROLE)
        .expect("Should have editor role");

    assert_eq!(member1.role_id, editor_role.id, "Member1 should have editor role");
}

#[tokio::test]
async fn test_workspace_deletion() {
    let test_app = TestApp::new("test_workspace_deletion").await;
    let mut conn = test_app.get_connection().await;

    // Create a workspace using comprehensive method
    let (user, _) = test_app.create_test_workspace_with_user().await.unwrap();

    let workspace_request = CreateWorkspaceRequest {
        name: format!("{}_delete_workspace", test_app.test_prefix()),
        owner_id: user.id,
    };
    let created_workspace = create_workspace(&mut conn, workspace_request).await.unwrap();

    // Verify workspace exists
    assert!(
        test_app.workspace_exists(&created_workspace.workspace.name).await.unwrap(),
        "Workspace should exist before deletion"
    );

    // Delete the workspace
    let result = delete_workspace(&mut conn, created_workspace.workspace.id).await;
    assert!(result.is_ok(), "Workspace deletion should succeed");

    // Verify workspace no longer exists
    let check_result = get_workspace(&mut conn, created_workspace.workspace.id).await;
    assert!(check_result.is_err(), "Workspace should not exist after deletion");
}

#[tokio::test]
async fn test_workspace_deletion_nonexistent() {
    let test_app = TestApp::new("test_workspace_deletion_nonexistent").await;
    let mut conn = test_app.get_connection().await;

    // Test deleting non-existent workspace
    let fake_id = uuid::Uuid::now_v7();
    let result = delete_workspace(&mut conn, fake_id).await;
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

#[tokio::test]
async fn test_workspace_creation_with_duplicate_owner_as_member() {
    let test_app = TestApp::new("test_workspace_creation_with_duplicate_owner_as_member").await;
    let mut conn = test_app.get_connection().await;

    // Create users for the test
    let (owner_user, _) = test_app.create_test_workspace_with_user().await.unwrap();
    let member_data = test_app.generate_test_user();
    let member_user = backend::services::users::register_user(&mut conn, member_data).await.unwrap();

    // Try to create workspace with owner also listed in members (should be handled gracefully)
    let workspace_request = CreateWorkspaceWithMembersRequest {
        name: format!("{}_duplicate_owner_test", test_app.test_prefix()),
        owner_id: owner_user.id,
        members: vec![
            WorkspaceMemberRequest {
                user_id: owner_user.id, // Owner listed as member
                role_name: ADMIN_ROLE.to_string(),
            },
            WorkspaceMemberRequest {
                user_id: member_user.id,
                role_name: EDITOR_ROLE.to_string(),
            },
        ],
    };

    let result = create_workspace_with_members(&mut conn, workspace_request).await;
    assert!(result.is_ok(), "Should handle duplicate owner gracefully");

    let workspace_result = result.unwrap();

    // Should only have 2 members (owner + member_user), not 3
    assert_eq!(workspace_result.members.len(), 2, "Should deduplicate owner");

    // Verify both users are present
    let member_user_ids: Vec<_> = workspace_result.members.iter().map(|m| m.user_id).collect();
    assert!(member_user_ids.contains(&owner_user.id), "Owner should be present");
    assert!(member_user_ids.contains(&member_user.id), "Other member should be present");
}