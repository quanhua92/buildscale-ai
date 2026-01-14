use crate::common::{TestApp, TestAppOptions, create_workspace, generate_test_email, register_and_login};

// ============================================================================
// CREATE WORKSPACE TESTS
// ============================================================================

#[tokio::test]
async fn test_create_workspace_returns_200_on_success() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;

    let response = app
        .client
        .post(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": "Test Workspace"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["workspace"].is_object());
    assert_eq!(body["workspace"]["name"], "Test Workspace");
    assert!(body["roles"].is_array());
    assert!(body["owner_membership"].is_object());
}

#[tokio::test]
async fn test_create_workspace_returns_401_without_token() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;

    let response = app
        .client
        .post(&app.url("/api/v1/workspaces"))
        .json(&serde_json::json!({
            "name": "Test Workspace"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_create_workspace_validates_empty_name() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;

    let response = app
        .client
        .post(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_create_workspace_validates_long_name() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;

    let long_name = "a".repeat(101);
    let response = app
        .client
        .post(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": long_name
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

// ============================================================================
// LIST WORKSPACES TESTS
// ============================================================================

#[tokio::test]
async fn test_list_workspaces_returns_200_with_token() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;

    let response = app
        .client
        .get(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["workspaces"].is_array());
    assert!(body["count"].is_number());
}

#[tokio::test]
async fn test_list_workspaces_returns_401_without_token() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;

    let response = app
        .client
        .get(&app.url("/api/v1/workspaces"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_list_workspaces_filters_by_user() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;

    // User1 creates a workspace
    let token1 = register_and_login(&app).await;
    let workspace1_id = create_workspace(&app, &token1, "User1 Workspace").await;

    // User2 creates a workspace
    let token2 = register_and_login(&app).await;
    let _workspace2_id = create_workspace(&app, &token2, "User2 Workspace").await;

    // User1 lists workspaces - should only see their own
    let response = app
        .client
        .get(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", token1))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let workspaces = body["workspaces"].as_array().unwrap();
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0]["id"], workspace1_id);
}

// ============================================================================
// GET SINGLE WORKSPACE TESTS
// ============================================================================

#[tokio::test]
async fn test_get_workspace_returns_200_for_owner() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Test Workspace").await;

    let response = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["workspace"]["id"], workspace_id);
    assert_eq!(body["workspace"]["name"], "Test Workspace");
}

#[tokio::test]
async fn test_get_workspace_returns_401_without_token() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Test Workspace").await;

    let response = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_get_workspace_returns_403_for_non_member() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;

    // User1 creates a workspace
    let token1 = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token1, "Private Workspace").await;

    // User2 tries to access User1's workspace
    let token2 = register_and_login(&app).await;

    let response = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .header("Authorization", format!("Bearer {}", token2))
        .send()
        .await
        .unwrap();

    // 403 (not 404) - prevents workspace enumeration
    assert_eq!(response.status(), 403);
}

// ============================================================================
// UPDATE WORKSPACE TESTS
// ============================================================================

#[tokio::test]
async fn test_update_workspace_returns_200_for_owner() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Original Name").await;

    let response = app
        .client
        .patch(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": "Updated Name"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["workspace"]["name"], "Updated Name");
}

#[tokio::test]
async fn test_update_workspace_returns_401_without_token() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Test Workspace").await;

    let response = app
        .client
        .patch(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .json(&serde_json::json!({
            "name": "Hacked Name"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_update_workspace_returns_403_for_non_member() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;

    // User1 creates a workspace
    let token1 = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token1, "Owner Workspace").await;

    // User2 tries to update User1's workspace
    let token2 = register_and_login(&app).await;

    let response = app
        .client
        .patch(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .header("Authorization", format!("Bearer {}", token2))
        .json(&serde_json::json!({
            "name": "Hacked Name"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn test_update_workspace_validates_empty_name() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Test Workspace").await;

    let response = app
        .client
        .patch(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

// ============================================================================
// DELETE WORKSPACE TESTS
// ============================================================================

#[tokio::test]
async fn test_delete_workspace_returns_200_for_owner() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "To Delete").await;

    let response = app
        .client
        .delete(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Verify workspace is deleted
    let get_response = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(get_response.status(), 404);
}

#[tokio::test]
async fn test_delete_workspace_returns_401_without_token() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Test Workspace").await;

    let response = app
        .client
        .delete(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_delete_workspace_returns_403_for_non_member() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;

    // User1 creates a workspace
    let token1 = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token1, "Owner Workspace").await;

    // User2 tries to delete User1's workspace
    let token2 = register_and_login(&app).await;

    let response = app
        .client
        .delete(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .header("Authorization", format!("Bearer {}", token2))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 403);

    // Verify workspace still exists for owner
    let get_response = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}", workspace_id)))
        .header("Authorization", format!("Bearer {}", token1))
        .send()
        .await
        .unwrap();

    assert_eq!(get_response.status(), 200);
}
