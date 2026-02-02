//! Tests for model update functionality
//!
//! These tests verify that:
//! - Model is persisted correctly in chat session
//! - Model can be changed mid-chat
//! - Model changes persist across page reloads
//! - New messages use the updated model

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::chat::{create_chat, get_chat, post_message};

#[tokio::test]
async fn test_chat_created_with_default_model() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Model Test").await;

    // Create chat without specifying model
    let chat_id = create_chat(&app, &workspace_id, &token, "Hello").await;

    // Verify default model is openai:gpt-5-mini (multi-provider format includes provider prefix)
    let chat = get_chat(&app, &workspace_id, &chat_id, &token).await;
    assert_eq!(chat["agent_config"]["model"], "openai:gpt-5-mini");
}

#[tokio::test]
async fn test_chat_created_with_explicit_model() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Model Test").await;

    // Create chat with gpt-4o
    let response = app
        .client
        .post(&format!(
            "{}/api/v1/workspaces/{}/chats",
            app.address, workspace_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "goal": "Use gpt-4o",
            "model": "gpt-4o"
        }))
        .send()
        .await
        .expect("Failed to create chat");

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    let chat_id = body["chat_id"].as_str().unwrap();

    // Verify model is gpt-4o
    let chat = get_chat(&app, &workspace_id, chat_id, &token).await;
    assert_eq!(chat["agent_config"]["model"], "openai:gpt-4o");
}

#[tokio::test]
async fn test_model_updated_on_message() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Model Test").await;

    // Create chat with default model
    let chat_id = create_chat(&app, &workspace_id, &token, "Start with mini").await;

    let chat = get_chat(&app, &workspace_id, &chat_id, &token).await;
    assert_eq!(chat["agent_config"]["model"], "openai:gpt-5-mini");

    // Send message with model change to gpt-4o
    let response = post_message(
        &app,
        &workspace_id,
        &chat_id,
        &token,
        "Switch to gpt-4o",
        Some("gpt-4o"),
    )
    .await;
    assert_eq!(response.status(), 202); // Accepted

    // Verify model updated
    let chat = get_chat(&app, &workspace_id, &chat_id, &token).await;
    assert_eq!(chat["agent_config"]["model"], "openai:gpt-4o");
}

#[tokio::test]
async fn test_model_persists_across_messages() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Model Test").await;

    // Create chat with gpt-4o
    let chat_id = create_chat(&app, &workspace_id, &token, "Start").await;

    // Switch to gpt-4o-mini
    post_message(
        &app,
        &workspace_id,
        &chat_id,
        &token,
        "Switch to mini",
        Some("gpt-4o-mini"),
    )
    .await;

    // Send multiple messages without specifying model
    for i in 1..=3 {
        let response = post_message(
            &app,
            &workspace_id,
            &chat_id,
            &token,
            &format!("Message {}", i),
            None, // Don't specify model
        )
        .await;
        assert_eq!(response.status(), 202);
    }

    // Verify model persisted as gpt-4o-mini
    let chat = get_chat(&app, &workspace_id, &chat_id, &token).await;
    assert_eq!(chat["agent_config"]["model"], "openai:gpt-4o-mini");
}

#[tokio::test]
async fn test_model_can_be_switched_multiple_times() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Model Test").await;

    let chat_id = create_chat(&app, &workspace_id, &token, "Start").await;

    // Mini -> 4o
    post_message(&app, &workspace_id, &chat_id, &token, "Switch 1", Some("gpt-4o")).await;
    let chat = get_chat(&app, &workspace_id, &chat_id, &token).await;
    assert_eq!(chat["agent_config"]["model"], "openai:gpt-4o");

    // 4o -> Mini
    post_message(
        &app,
        &workspace_id,
        &chat_id,
        &token,
        "Switch 2",
        Some("gpt-4o-mini"),
    )
    .await;
    let chat = get_chat(&app, &workspace_id, &chat_id, &token).await;
    assert_eq!(chat["agent_config"]["model"], "openai:gpt-4o-mini");

    // Mini -> 4o again
    post_message(&app, &workspace_id, &chat_id, &token, "Switch 3", Some("gpt-4o")).await;
    let chat = get_chat(&app, &workspace_id, &chat_id, &token).await;
    assert_eq!(chat["agent_config"]["model"], "openai:gpt-4o");
}
