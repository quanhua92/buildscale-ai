//! Tests for chat persistence and audit trail functionality.
//!
//! These tests verify that all streaming events (reasoning chunks, tool calls,
//! tool results) are correctly persisted to the database with proper metadata.

use crate::common::database::TestApp;
use buildscale::{
    load_config,
    models::chat::{ChatMessageMetadata, ChatMessageRole},
    models::files::FileType,
    models::requests::CreateFileRequest,
    services::chat::ChatService,
    services::files::create_file_with_content,
    services::storage::FileStorageService,
};
use uuid::Uuid;
use std::sync::Arc;

/// Helper to create a chat file within the given test app.
async fn setup_chat(test_app: &TestApp) -> (Uuid, Uuid) {
    let mut conn = test_app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file
    let chat_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "test_chat".to_string(),
        slug: None,
        path: None,
        is_virtual: Some(true),
        is_remote: None,
        permission: None,
        file_type: FileType::Chat,
        content: serde_json::json!({}),
        app_data: None,
    };
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
        .await
        .expect("Failed to create chat file");

    (workspace.id, chat.file.id)
}

#[tokio::test]
async fn test_save_reasoning_complete_persists_metadata() {
    let test_app = TestApp::new("test_save_reasoning_complete_persists_metadata").await;
    let (workspace_id, chat_id) = setup_chat(&test_app).await;
    let mut conn = test_app.get_connection().await;
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    // Save a reasoning complete message (aggregated)
    let metadata = ChatMessageMetadata {
        message_type: Some("reasoning_complete".to_string()),
        reasoning_id: Some(Uuid::new_v4().to_string()),
        ..Default::default()
    };

    let result = ChatService::save_stream_event(
        &mut conn,
        &storage,
        workspace_id,
        chat_id,
        ChatMessageRole::Assistant,
        "The user wants to build a todo app.".to_string(),
        metadata.clone(),
    ).await;

    assert!(result.is_ok());
    let msg = result.unwrap();

    // Verify the message was saved with correct role and metadata
    assert_eq!(msg.role, ChatMessageRole::Assistant);
    assert_eq!(msg.metadata.message_type, Some("reasoning_complete".to_string()));
    assert!(msg.metadata.reasoning_id.is_some());
    assert_eq!(msg.content, "The user wants to build a todo app.");
}

#[tokio::test]
async fn test_save_tool_call_persists_arguments() {
    let test_app = TestApp::new("test_save_tool_call_persists_arguments").await;
    let (workspace_id, chat_id) = setup_chat(&test_app).await;
    let mut conn = test_app.get_connection().await;
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    let tool_name = "write".to_string();
    let arguments = serde_json::json!({
        "path": "/src/App.tsx",
        "content": "import React from 'react';"
    });

    let metadata = ChatMessageMetadata {
        message_type: Some("tool_call".to_string()),
        tool_name: Some(tool_name.clone()),
        tool_arguments: Some(arguments.clone()),
        ..Default::default()
    };

    let result = ChatService::save_stream_event(
        &mut conn,
        &storage,
        workspace_id,
        chat_id,
        ChatMessageRole::Tool,
        format!("AI called tool: {}", tool_name),
        metadata,
    ).await;

    assert!(result.is_ok());
    let msg = result.unwrap();

    assert_eq!(msg.role, ChatMessageRole::Tool);
    assert_eq!(msg.metadata.message_type, Some("tool_call".to_string()));
    assert_eq!(msg.metadata.tool_name, Some(tool_name));
    assert_eq!(msg.metadata.tool_arguments, Some(arguments));
}

#[tokio::test]
async fn test_save_tool_result_persists_output_and_success() {
    let test_app = TestApp::new("test_save_tool_result_persists_output_and_success").await;
    let (workspace_id, chat_id) = setup_chat(&test_app).await;
    let mut conn = test_app.get_connection().await;
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    let tool_name = "ls".to_string();
    let output = "src\npackage.json\nREADME.md".to_string();
    let success = true;

    let metadata = ChatMessageMetadata {
        message_type: Some("tool_result".to_string()),
        tool_name: Some(tool_name.clone()),
        tool_output: Some(output.clone()),
        tool_success: Some(success),
        ..Default::default()
    };

    let result = ChatService::save_stream_event(
        &mut conn,
        &storage,
        workspace_id,
        chat_id,
        ChatMessageRole::Tool,
        format!("Tool {}: succeeded", tool_name),
        metadata,
    ).await;

    assert!(result.is_ok());
    let msg = result.unwrap();

    assert_eq!(msg.role, ChatMessageRole::Tool);
    assert_eq!(msg.metadata.message_type, Some("tool_result".to_string()));
    assert_eq!(msg.metadata.tool_name, Some(tool_name));
    assert_eq!(msg.metadata.tool_output, Some(output));
    assert_eq!(msg.metadata.tool_success, Some(true));
}

#[tokio::test]
async fn test_tool_call_and_result_share_reasoning_id() {
    let test_app = TestApp::new("test_tool_call_and_result_share_reasoning_id").await;
    let (workspace_id, chat_id) = setup_chat(&test_app).await;
    let mut conn = test_app.get_connection().await;
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    let reasoning_id = Uuid::new_v4().to_string();

    // 1. Save tool call linked to reasoning_id
    let call_meta = ChatMessageMetadata {
        message_type: Some("tool_call".to_string()),
        reasoning_id: Some(reasoning_id.clone()),
        tool_name: Some("ls".to_string()),
        ..Default::default()
    };
    ChatService::save_stream_event(
        &mut conn,
        &storage,
        workspace_id,
        chat_id,
        ChatMessageRole::Tool,
        "AI called tool: ls".to_string(),
        call_meta,
    ).await.unwrap();

    // 2. Save tool result linked to same reasoning_id
    let result_meta = ChatMessageMetadata {
        message_type: Some("tool_result".to_string()),
        reasoning_id: Some(reasoning_id.clone()),
        tool_name: Some("ls".to_string()),
        tool_success: Some(true),
        ..Default::default()
    };
    ChatService::save_stream_event(
        &mut conn,
        &storage,
        workspace_id,
        chat_id,
        ChatMessageRole::Tool,
        "Tool ls: succeeded".to_string(),
        result_meta,
    ).await.unwrap();

    // Query messages back
    let messages = buildscale::queries::chat::get_messages_by_file_id(&mut conn, workspace_id, chat_id)
        .await
        .unwrap();

    assert_eq!(messages.len(), 2);
    for msg in messages {
        assert_eq!(msg.metadata.reasoning_id, Some(reasoning_id.clone()));
    }
}
