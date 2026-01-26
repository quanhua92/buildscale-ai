//! Chat stop endpoint tests
//!
//! Tests for the POST /api/v1/workspaces/:id/chats/:chat_id/stop endpoint.

use crate::common::TestApp;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Generate a unique email for testing to avoid conflicts
fn generate_test_email() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("test_chat_stop_{}@example.com", timestamp)
}

async fn setup_test_chat(app: &TestApp) -> (String, String, Uuid, Uuid) {
    let email = generate_test_email();
    let password = "TestSecurePass123!";

    // Register user
    let _ = app
        .client
        .post(&app.url("/api/v1/auth/register"))
        .json(&json!({
            "email": email,
            "password": password,
            "confirm_password": password,
            "full_name": "Test User"
        }))
        .send()
        .await
        .unwrap();

    // Login to get access token
    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .unwrap();

    let login_data: serde_json::Value = login_response.json().await.unwrap();
    let access_token = login_data["access_token"].as_str().unwrap();

    // Create workspace
    let workspace_response = app
        .client
        .post(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({
            "name": "Test Workspace"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(workspace_response.status(), 200);

    let workspace_data: serde_json::Value = workspace_response.json().await.unwrap();
    let workspace_id = Uuid::parse_str(workspace_data["workspace"]["id"].as_str().unwrap()).unwrap();

    // Create a chat
    let chat_response = app
        .client
        .post(&app.url(&format!(
            "/api/v1/workspaces/{}/chats",
            workspace_id
        )))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({
            "goal": "Test chat for stop endpoint"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(chat_response.status(), 201);

    let chat_data: serde_json::Value = chat_response.json().await.unwrap();
    let chat_id = chat_data["chat_id"].as_str().unwrap();

    (email.to_string(), access_token.to_string(), workspace_id, Uuid::parse_str(chat_id).unwrap())
}

// ============================================================================
// Stop Endpoint Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_stop_endpoint_requires_authentication() {
    let app = TestApp::new().await;
    let workspace_id = Uuid::new_v4();
    let chat_id = Uuid::new_v4();

    let response = app
        .client
        .post(&app.url(&format!(
            "/api/v1/workspaces/{}/chats/{}/stop",
            workspace_id, chat_id
        )))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_stop_endpoint_with_invalid_token_returns_401() {
    let app = TestApp::new().await;
    let workspace_id = Uuid::new_v4();
    let chat_id = Uuid::new_v4();

    let response = app
        .client
        .post(&app.url(&format!(
            "/api/v1/workspaces/{}/chats/{}/stop",
            workspace_id, chat_id
        )))
        .header("Authorization", "Bearer invalid.jwt.token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_stop_nonexistent_chat_returns_404() {
    let app = TestApp::new().await;
    let email = generate_test_email();
    let password = "TestSecurePass123!";

    // Register and login
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&json!({
            "email": email,
            "password": password,
            "confirm_password": password,
            "full_name": "Test User"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .unwrap();

    let login_data: serde_json::Value = login_response.json().await.unwrap();
    let access_token = login_data["access_token"].as_str().unwrap();

    // Create workspace
    let workspace_response = app
        .client
        .post(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({
            "name": "Test Workspace"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(workspace_response.status(), 200);

    let workspace_data: serde_json::Value = workspace_response.json().await.unwrap();
    let workspace_id = Uuid::parse_str(workspace_data["workspace"]["id"].as_str().unwrap()).unwrap();

    // Try to stop a chat that doesn't exist
    let fake_chat_id = Uuid::new_v4();
    let response = app
        .client
        .post(&app.url(&format!(
            "/api/v1/workspaces/{}/chats/{}/stop",
            workspace_id, fake_chat_id
        )))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "NOT_FOUND");
}

// ============================================================================
// Stop Endpoint Functionality Tests
// ============================================================================

#[tokio::test]
async fn test_stop_endpoint_returns_200() {
    let app = TestApp::new().await;

    let (_email, access_token, workspace_id, chat_id) = setup_test_chat(&app).await;

    let response = app
        .client
        .post(&app.url(&format!(
            "/api/v1/workspaces/{}/chats/{}/stop",
            workspace_id, chat_id
        )))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "cancelled");
    assert_eq!(body["chat_id"], chat_id.to_string());
}

#[tokio::test]
async fn test_stop_endpoint_multiple_requests() {
    let app = TestApp::new().await;

    let (_email, access_token, workspace_id, chat_id) = setup_test_chat(&app).await;

    // Send multiple stop requests (should be idempotent)
    for _ in 0..3 {
        let response = app
            .client
            .post(&app.url(&format!(
                "/api/v1/workspaces/{}/chats/{}/stop",
                workspace_id, chat_id
            )))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}

#[tokio::test]
async fn test_stop_endpoint_returns_correct_content_type() {
    let app = TestApp::new().await;

    let (_email, access_token, workspace_id, chat_id) = setup_test_chat(&app).await;

    let response = app
        .client
        .post(&app.url(&format!(
            "/api/v1/workspaces/{}/chats/{}/stop",
            workspace_id, chat_id
        )))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let content_type = response.headers().get("content-type").unwrap();
    assert!(
        content_type.to_str().unwrap().contains("application/json"),
        "Response should be JSON"
    );
}

// ============================================================================
// Stop Endpoint Integration Tests
// ============================================================================

#[tokio::test]
async fn test_stop_endpoint_with_wrong_workspace_returns_403_or_404() {
    let app = TestApp::new().await;
    let other_workspace_id = Uuid::new_v4();

    let (_email, access_token, _workspace_id, chat_id) = setup_test_chat(&app).await;

    // Try to stop a chat in a different workspace
    let response = app
        .client
        .post(&app.url(&format!(
            "/api/v1/workspaces/{}/chats/{}/stop",
            other_workspace_id, chat_id
        )))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .unwrap();

    // Should return 403 (Forbidden) or 404 (Not Found) depending on middleware order
    assert!(response.status() == 403 || response.status() == 404);
}

#[tokio::test]
async fn test_get_chat_endpoint() {
    let app = TestApp::new().await;
    let (_email, access_token, workspace_id, chat_id) = setup_test_chat(&app).await;

    // Post a message first
    let msg_content = "Hello History";
    let post_response = app
        .client
        .post(&app.url(&format!(
            "/api/v1/workspaces/{}/chats/{}",
            workspace_id, chat_id
        )))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({
            "content": msg_content
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(post_response.status(), 202);

    // Get chat history
    let response = app
        .client
        .get(&app.url(&format!(
            "/api/v1/workspaces/{}/chats/{}",
            workspace_id, chat_id
        )))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    
    // Verify structure
    assert_eq!(body["file_id"], chat_id.to_string());
    assert!(body["agent_config"].is_object());
    assert!(body["messages"].is_array());
    
    let messages = body["messages"].as_array().unwrap();
    // 1 initial goal + 1 posted message = 2 messages
    assert!(messages.len() >= 2);
    
    // Check if our posted message is there
    let found = messages.iter().any(|m| m["content"].as_str() == Some(msg_content));
    assert!(found, "Posted message not found in history");
}
