use backend::{
    services::users::register_user_with_workspace,
    models::requests::UserWorkspaceRegistrationRequest,
    services::workspaces::get_workspace,
    services::roles::list_workspace_roles,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_user_registration_with_workspace_success() {
    let test_app = TestApp::new("test_user_registration_with_workspace_success").await;
    let mut conn = test_app.get_connection().await;

    let initial_user_count = test_app.count_test_users().await.unwrap();
    let initial_workspace_count = test_app.count_test_workspaces().await.unwrap();

    let registration_request = UserWorkspaceRegistrationRequest {
        email: test_app.generate_test_email(),
        password: "testpassword123".to_string(),
        confirm_password: "testpassword123".to_string(),
        full_name: Some("Test User with Workspace".to_string()),
        workspace_name: format!("{}_workspace", test_app.test_prefix()),
    };

    let result = register_user_with_workspace(&mut conn, registration_request).await;
    assert!(result.is_ok(), "User registration with workspace should succeed");

    let registration_result = result.unwrap();

    // Verify user was created
    assert!(!registration_result.user.id.to_string().is_empty(), "User should have a valid UUID");
    assert!(registration_result.user.email.starts_with(&test_app.test_prefix()), "User email should have test prefix");
    assert!(registration_result.user.email.ends_with("@example.com"), "User email should end with @example.com");
    assert_eq!(registration_result.user.full_name, Some("Test User with Workspace".to_string()));

    // Verify workspace was created
    assert!(!registration_result.workspace.workspace.id.to_string().is_empty(), "Workspace should have a valid UUID");
    assert_eq!(registration_result.workspace.workspace.owner_id, registration_result.user.id, "User should be workspace owner");
    assert_eq!(
        registration_result.workspace.workspace.name,
        format!("{}_workspace", test_app.test_prefix())
    );

    // Verify default roles were created
    assert_eq!(registration_result.workspace.roles.len(), 3, "Should create 3 default roles");

    // Verify owner was added as admin member
    assert_eq!(
        registration_result.workspace.owner_membership.user_id,
        registration_result.user.id,
        "User should be added as admin member"
    );

    let final_user_count = test_app.count_test_users().await.unwrap();
    let final_workspace_count = test_app.count_test_workspaces().await.unwrap();

    assert_eq!(final_user_count, initial_user_count + 1, "Should create 1 new user");
    assert_eq!(final_workspace_count, initial_workspace_count + 1, "Should create 1 new workspace");
}

#[tokio::test]
async fn test_user_registration_with_workspace_password_mismatch() {
    let test_app = TestApp::new("test_user_registration_with_workspace_password_mismatch").await;
    let mut conn = test_app.get_connection().await;

    let registration_request = UserWorkspaceRegistrationRequest {
        email: test_app.generate_test_email(),
        password: "testpassword123".to_string(),
        confirm_password: "differentpassword".to_string(),
        full_name: Some("Test User".to_string()),
        workspace_name: format!("{}_workspace", test_app.test_prefix()),
    };

    let result = register_user_with_workspace(&mut conn, registration_request).await;
    assert!(result.is_err(), "Password mismatch should cause registration to fail");

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("Passwords do not match"),
        "Error message should mention password mismatch: {}",
        error_message
    );
}

#[tokio::test]
async fn test_user_registration_with_workspace_short_password() {
    let test_app = TestApp::new("test_user_registration_with_workspace_short_password").await;
    let mut conn = test_app.get_connection().await;

    let registration_request = UserWorkspaceRegistrationRequest {
        email: test_app.generate_test_email(),
        password: "short".to_string(),
        confirm_password: "short".to_string(),
        full_name: Some("Test User".to_string()),
        workspace_name: format!("{}_workspace", test_app.test_prefix()),
    };

    let result = register_user_with_workspace(&mut conn, registration_request).await;
    assert!(result.is_err(), "Short password should cause registration to fail");

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("8 characters"),
        "Error message should mention password length: {}",
        error_message
    );
}

#[tokio::test]
async fn test_user_registration_with_workspace_empty_workspace_name() {
    let test_app = TestApp::new("test_user_registration_with_workspace_empty_workspace_name").await;
    let mut conn = test_app.get_connection().await;

    let registration_request = UserWorkspaceRegistrationRequest {
        email: test_app.generate_test_email(),
        password: "testpassword123".to_string(),
        confirm_password: "testpassword123".to_string(),
        full_name: Some("Test User".to_string()),
        workspace_name: String::new(),
    };

    let result = register_user_with_workspace(&mut conn, registration_request).await;
    assert!(result.is_err(), "Empty workspace name should cause registration to fail");

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("empty"),
        "Error message should mention empty workspace name: {}",
        error_message
    );
}

#[tokio::test]
async fn test_user_registration_with_workspace_long_workspace_name() {
    let test_app = TestApp::new("test_user_registration_with_workspace_long_workspace_name").await;
    let mut conn = test_app.get_connection().await;

    let registration_request = UserWorkspaceRegistrationRequest {
        email: test_app.generate_test_email(),
        password: "testpassword123".to_string(),
        confirm_password: "testpassword123".to_string(),
        full_name: Some("Test User".to_string()),
        workspace_name: "a".repeat(101), // Too long
    };

    let result = register_user_with_workspace(&mut conn, registration_request).await;
    assert!(result.is_err(), "Long workspace name should cause registration to fail");

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("100") || error_message.contains("length"),
        "Error message should mention length requirement: {}",
        error_message
    );
}

#[tokio::test]
async fn test_user_registration_with_workspace_whitespace_workspace_name() {
    let test_app = TestApp::new("test_user_registration_with_workspace_whitespace_workspace_name").await;
    let mut conn = test_app.get_connection().await;

    let registration_request = UserWorkspaceRegistrationRequest {
        email: test_app.generate_test_email(),
        password: "testpassword123".to_string(),
        confirm_password: "testpassword123".to_string(),
        full_name: Some("Test User".to_string()),
        workspace_name: "   ".to_string(),
    };

    let result = register_user_with_workspace(&mut conn, registration_request).await;
    assert!(result.is_err(), "Whitespace-only workspace name should cause registration to fail");

    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("empty"),
        "Error message should mention empty workspace name: {}",
        error_message
    );
}

#[tokio::test]
async fn test_user_registration_with_workspace_max_valid_workspace_name() {
    let test_app = TestApp::new("test_user_registration_with_workspace_max_valid_workspace_name").await;
    let mut conn = test_app.get_connection().await;

    let registration_request = UserWorkspaceRegistrationRequest {
        email: test_app.generate_test_email(),
        password: "testpassword123".to_string(),
        confirm_password: "testpassword123".to_string(),
        full_name: Some("Test User".to_string()),
        workspace_name: "a".repeat(100), // Exactly 100 characters - should succeed
    };

    let result = register_user_with_workspace(&mut conn, registration_request).await;
    assert!(result.is_ok(), "100-character workspace name should be valid");

    let registration_result = result.unwrap();
    assert_eq!(registration_result.workspace.workspace.name.len(), 100, "Workspace name should be 100 characters");
}

#[tokio::test]
async fn test_user_registration_with_workspace_default_roles_created() {
    let test_app = TestApp::new("test_user_registration_with_workspace_default_roles_created").await;
    let mut conn = test_app.get_connection().await;

    let registration_request = UserWorkspaceRegistrationRequest {
        email: test_app.generate_test_email(),
        password: "testpassword123".to_string(),
        confirm_password: "testpassword123".to_string(),
        full_name: Some("Test User".to_string()),
        workspace_name: format!("{}_workspace", test_app.test_prefix()),
    };

    let result = register_user_with_workspace(&mut conn, registration_request).await;
    assert!(result.is_ok(), "Registration should succeed");

    let registration_result = result.unwrap();

    // Verify all three default roles were created
    assert_eq!(registration_result.workspace.roles.len(), 3, "Should create 3 default roles");

    let role_names: Vec<String> = registration_result.workspace.roles.iter().map(|r| r.name.clone()).collect();
    assert!(role_names.contains(&"admin".to_string()), "Should have admin role");
    assert!(role_names.contains(&"editor".to_string()), "Should have editor role");
    assert!(role_names.contains(&"viewer".to_string()), "Should have viewer role");

    // Verify owner was added with admin role
    let admin_role = registration_result.workspace.roles.iter()
        .find(|r| r.name == "admin")
        .expect("Should have admin role");

    assert_eq!(
        registration_result.workspace.owner_membership.role_id,
        admin_role.id,
        "Owner should have admin role"
    );
}

#[tokio::test]
async fn test_user_registration_with_workspace_workspace_accessible() {
    let test_app = TestApp::new("test_user_registration_with_workspace_workspace_accessible").await;
    let mut conn = test_app.get_connection().await;

    let registration_request = UserWorkspaceRegistrationRequest {
        email: test_app.generate_test_email(),
        password: "testpassword123".to_string(),
        confirm_password: "testpassword123".to_string(),
        full_name: Some("Test User".to_string()),
        workspace_name: format!("{}_workspace", test_app.test_prefix()),
    };

    let result = register_user_with_workspace(&mut conn, registration_request).await;
    assert!(result.is_ok(), "Registration should succeed");

    let registration_result = result.unwrap();

    // Verify workspace can be retrieved from database
    let retrieved_workspace = get_workspace(&mut conn, registration_result.workspace.workspace.id).await;
    assert!(retrieved_workspace.is_ok(), "Workspace should be accessible");

    let workspace = retrieved_workspace.unwrap();
    assert_eq!(workspace.id, registration_result.workspace.workspace.id, "Workspace ID should match");
    assert_eq!(workspace.name, registration_result.workspace.workspace.name, "Workspace name should match");

    // Verify roles can be listed from database
    let roles = list_workspace_roles(&mut conn, workspace.id).await;
    assert!(roles.is_ok(), "Roles should be accessible");

    let role_list = roles.unwrap();
    assert_eq!(role_list.len(), 3, "Should have 3 roles in database");
}