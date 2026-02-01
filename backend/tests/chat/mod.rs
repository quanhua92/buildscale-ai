//! Chat service tests
//!
//! Tests for chat model management and persistence.

pub mod model_update_tests;
pub mod yaml_sync_tests;

use crate::common::TestApp;

/// Helper: Create a new chat session
pub async fn create_chat(
    app: &TestApp,
    workspace_id: &str,
    token: &str,
    goal: &str,
) -> String {
    let response = app
        .client
        .post(&format!("{}/api/v1/workspaces/{}/chats", app.address, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "goal": goal }))
        .send()
        .await
        .expect("Failed to create chat");

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    body["chat_id"].as_str().unwrap().to_string()
}

/// Helper: Get chat session details
pub async fn get_chat(
    app: &TestApp,
    workspace_id: &str,
    chat_id: &str,
    token: &str,
) -> serde_json::Value {
    let response = app
        .client
        .get(&format!(
            "{}/api/v1/workspaces/{}/chats/{}",
            app.address, workspace_id, chat_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get chat");

    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// Helper: Post message to chat
pub async fn post_message(
    app: &TestApp,
    workspace_id: &str,
    chat_id: &str,
    token: &str,
    content: &str,
    model: Option<&str>,
) -> reqwest::Response {
    let mut payload = serde_json::json!({ "content": content });
    if let Some(m) = model {
        payload["model"] = serde_json::json!(m);
    }

    app.client
        .post(&format!(
            "{}/api/v1/workspaces/{}/chats/{}",
            app.address, workspace_id, chat_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&payload)
        .send()
        .await
        .expect("Failed to post message")
}
