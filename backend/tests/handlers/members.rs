use crate::common::{TestApp, TestAppOptions, create_workspace, generate_test_email, register_and_login};

// ============================================================================
// LIST MEMBERS TESTS
// ============================================================================

#[tokio::test]
async fn test_list_members_returns_200_for_member() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Member List Test").await;

    let response = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}/members", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let members = body["members"].as_array().unwrap();
    assert!(!members.is_empty());
    
    // Owner should be in the list
    let owner_member = members.iter().find(|m| m["role_name"] == "admin").unwrap();
    assert!(owner_member["email"].is_string());
}

#[tokio::test]
async fn test_list_members_returns_401_without_token() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Auth Test").await;

    let response = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}/members", workspace_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_list_members_returns_403_for_non_member() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    
    let token1 = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token1, "Private Workspace").await;

    let token2 = register_and_login(&app).await;

    let response = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}/members", workspace_id)))
        .header("Authorization", format!("Bearer {}", token2))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 403);
}

// ============================================================================
// GET MY MEMBERSHIP TESTS
// ============================================================================

#[tokio::test]
async fn test_get_my_membership_returns_200() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "My Membership Test").await;

    let response = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}/members/me", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["member"]["role_name"], "admin"); // Creator is Admin
}

// ============================================================================
// ADD MEMBER TESTS
// ============================================================================

#[tokio::test]
async fn test_add_member_returns_200_on_success() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    
    // Admin creates workspace
    let token_admin = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token_admin, "Team Workspace").await;

    // Register user to be added
    let user_email = generate_test_email();
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": user_email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    // Add user as Member
    let response = app
        .client
        .post(&app.url(&format!("/api/v1/workspaces/{}/members", workspace_id)))
        .header("Authorization", format!("Bearer {}", token_admin))
        .json(&serde_json::json!({
            "email": user_email,
            "role_name": "member"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["member"]["email"], user_email);
    assert_eq!(body["member"]["role_name"], "member");
}

#[tokio::test]
async fn test_add_member_returns_404_for_nonexistent_user() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Not Found Test").await;

    let response = app
        .client
        .post(&app.url(&format!("/api/v1/workspaces/{}/members", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "email": "nonexistent@example.com",
            "role_name": "Member"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}

// ============================================================================
// UPDATE MEMBER ROLE TESTS
// ============================================================================

#[tokio::test]
async fn test_update_member_role_returns_200_for_admin() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    
    let token_admin = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token_admin, "Role Update Test").await;

    // Add a member first
    let user_email = generate_test_email();
    let reg_resp = app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": user_email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();
    let user_id = reg_resp.json::<serde_json::Value>().await.unwrap()["user"]["id"].as_str().unwrap().to_string();

    app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/members", workspace_id)))
        .header("Authorization", format!("Bearer {}", token_admin))
        .json(&serde_json::json!({
            "email": user_email,
            "role_name": "viewer"
        }))
        .send()
        .await
        .unwrap();

    // Update role to Editor
    let response = app
        .client
        .patch(&app.url(&format!("/api/v1/workspaces/{}/members/{}", workspace_id, user_id)))
        .header("Authorization", format!("Bearer {}", token_admin))
        .json(&serde_json::json!({
            "role_name": "editor"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["member"]["role_name"], "editor");
}

#[tokio::test]
async fn test_update_owner_role_returns_403() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Owner Update Test").await;

    // Get owner ID
    let me_resp = app.client
        .get(&app.url("/api/v1/auth/me"))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    let owner_id = me_resp.json::<serde_json::Value>().await.unwrap()["user"]["id"].as_str().unwrap().to_string();

    // Try to demote owner
    let response = app
        .client
        .patch(&app.url(&format!("/api/v1/workspaces/{}/members/{}", workspace_id, owner_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "role_name": "Viewer"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 403);
}

// ============================================================================
// REMOVE MEMBER TESTS
// ============================================================================

#[tokio::test]
async fn test_remove_member_returns_200_for_admin() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    
    let token_admin = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token_admin, "Removal Test").await;

    // Add a member
    let user_email = generate_test_email();
    let reg_resp = app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": user_email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();
    let user_id = reg_resp.json::<serde_json::Value>().await.unwrap()["user"]["id"].as_str().unwrap().to_string();

    app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/members", workspace_id)))
        .header("Authorization", format!("Bearer {}", token_admin))
        .json(&serde_json::json!({
            "email": user_email,
            "role_name": "member"
        }))
        .send()
        .await
        .unwrap();

    // Remove member
    let response = app
        .client
        .delete(&app.url(&format!("/api/v1/workspaces/{}/members/{}", workspace_id, user_id)))
        .header("Authorization", format!("Bearer {}", token_admin))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_leave_workspace_returns_200() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    
    let token_admin = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token_admin, "Leave Test").await;

    // Add a member
    let user_email = generate_test_email();
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": user_email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    // Login as the new member
    let login_resp = app.client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": user_email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();
    let login_body: serde_json::Value = login_resp.json().await.unwrap();
    let token_member = login_body["access_token"].as_str().unwrap().to_string();
    let user_id = login_body["user"]["id"].as_str().unwrap().to_string();

    // Admin adds the member
    app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/members", workspace_id)))
        .header("Authorization", format!("Bearer {}", token_admin))
        .json(&serde_json::json!({
            "email": user_email,
            "role_name": "member"
        }))
        .send()
        .await
        .unwrap();

    // Member leaves workspace
    let response = app
        .client
        .delete(&app.url(&format!("/api/v1/workspaces/{}/members/{}", workspace_id, user_id)))
        .header("Authorization", format!("Bearer {}", token_member))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
