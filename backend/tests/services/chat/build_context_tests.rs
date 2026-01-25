//! Integration tests for ChatService::build_context
//!
//! Tests that build_context correctly:
//! - Builds context with persona, history, and file attachments
//! - Applies priority-based fragment management
//! - Optimizes for token limits
//! - Handles edge cases (empty chat, large context, etc.)
//! - Enforces security (workspace isolation)

use buildscale::models::chat::{ChatAttachment, ChatMessageMetadata, ChatMessageRole};
use buildscale::models::files::FileType;
use buildscale::models::requests::CreateFileRequest;
use buildscale::queries::chat;
use buildscale::services::chat::{ChatService, DEFAULT_CONTEXT_TOKEN_LIMIT};
use buildscale::services::files::create_file_with_content;
use crate::common::database::TestApp;

#[tokio::test]
async fn test_build_context_with_persona() {
    let test_app = TestApp::new("test_build_context_with_persona").await;
    let mut conn = test_app.get_connection().await;

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
    let chat = create_file_with_content(&mut conn, chat_request)
        .await
        .expect("Failed to create chat file");

    let context = ChatService::build_context(&mut conn, workspace.id, chat.file.id)
        .await
        .expect("Failed to build context");

    // Should contain system persona
    assert!(context.contains("BuildScale AI"));
    assert!(context.contains("professional software engineering assistant"));
}

#[tokio::test]
async fn test_build_context_with_history() {
    let test_app = TestApp::new("test_build_context_with_history").await;
    let mut conn = test_app.get_connection().await;

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
    let chat = create_file_with_content(&mut conn, chat_request)
        .await
        .expect("Failed to create chat file");

    // Add some messages
    chat::insert_chat_message(
        &mut conn,
        buildscale::models::chat::NewChatMessage {
            file_id: chat.file.id,
            workspace_id: workspace.id,
            role: ChatMessageRole::User,
            content: "Hello, how are you?".to_string(),
            metadata: sqlx::types::Json(ChatMessageMetadata::default()),
        },
    )
    .await
    .expect("Failed to insert message");

    chat::insert_chat_message(
        &mut conn,
        buildscale::models::chat::NewChatMessage {
            file_id: chat.file.id,
            workspace_id: workspace.id,
            role: ChatMessageRole::Assistant,
            content: "I'm doing well, thank you!".to_string(),
            metadata: sqlx::types::Json(ChatMessageMetadata::default()),
        },
    )
    .await
    .expect("Failed to insert message");

    let context = ChatService::build_context(&mut conn, workspace.id, chat.file.id)
        .await
        .expect("Failed to build context");

    // Should contain conversation history
    assert!(context.contains("Conversation History:"));
    assert!(context.contains("User: Hello, how are you?"));
    assert!(context.contains("Assistant: I'm doing well, thank you!"));
}

#[tokio::test]
async fn test_build_context_with_file_attachments() {
    let test_app = TestApp::new("test_build_context_with_file_attachments").await;
    let mut conn = test_app.get_connection().await;

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
    let chat = create_file_with_content(&mut conn, chat_request)
        .await
        .expect("Failed to create chat file");

    // Create a workspace file
    let file_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "test.txt".to_string(),
        slug: None,
        path: Some("/test.txt".to_string()),
        is_virtual: None,
        is_remote: None,
        permission: None,
        file_type: FileType::Document,
        content: serde_json::json!({"text": "Hello World"}),
        app_data: None,
    };
    let file = create_file_with_content(&mut conn, file_request)
        .await
        .expect("Failed to create file");

    // Add a message with the file attachment
    chat::insert_chat_message(
        &mut conn,
        buildscale::models::chat::NewChatMessage {
            file_id: chat.file.id,
            workspace_id: workspace.id,
            role: ChatMessageRole::User,
            content: "Please read this file".to_string(),
            metadata: sqlx::types::Json(ChatMessageMetadata {
                attachments: vec![ChatAttachment::File {
                    file_id: file.file.id,
                    version_id: None,
                }],
                tool_calls: None,
                usage: None,
            }),
        },
    )
    .await
    .expect("Failed to insert message with attachment");

    let context = ChatService::build_context(&mut conn, workspace.id, chat.file.id)
        .await
        .expect("Failed to build context");

    // Should contain file context with XML markers
    assert!(context.contains("<file_context>"));
    assert!(context.contains("</file_context>"));
    assert!(context.contains("File: /test.txt"));
    assert!(context.contains("Hello World"));
}

#[tokio::test]
async fn test_build_context_workspace_isolation() {
    let test_app = TestApp::new("test_build_context_workspace_isolation").await;
    let mut conn = test_app.get_connection().await;

    let (user1, workspace1) = test_app.create_test_workspace_with_user().await.unwrap();
    let (_user2, workspace2) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file in workspace 1
    let chat_request = CreateFileRequest {
        workspace_id: workspace1.id,
        parent_id: None,
        author_id: user1.id,
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
    let chat = create_file_with_content(&mut conn, chat_request)
        .await
        .expect("Failed to create chat file");

    // Create a file in workspace 2
    let file_request = CreateFileRequest {
        workspace_id: workspace2.id,
        parent_id: None,
        author_id: user1.id, // Same user, different workspace
        name: "secret.txt".to_string(),
        slug: None,
        path: Some("/secret.txt".to_string()),
        is_virtual: None,
        is_remote: None,
        permission: None,
        file_type: FileType::Document,
        content: serde_json::json!({"text": "Secret data"}),
        app_data: None,
    };
    let file = create_file_with_content(&mut conn, file_request)
        .await
        .expect("Failed to create file in workspace 2");

    // Add a message with the file from different workspace
    chat::insert_chat_message(
        &mut conn,
        buildscale::models::chat::NewChatMessage {
            file_id: chat.file.id,
            workspace_id: workspace1.id,
            role: ChatMessageRole::User,
            content: "Please read this file".to_string(),
            metadata: sqlx::types::Json(ChatMessageMetadata {
                attachments: vec![ChatAttachment::File {
                    file_id: file.file.id,
                    version_id: None,
                }],
                tool_calls: None,
                usage: None,
            }),
        },
    )
    .await
    .expect("Failed to insert message with attachment");

    let context = ChatService::build_context(&mut conn, workspace1.id, chat.file.id)
        .await
        .expect("Failed to build context");

    // Should NOT contain the file from different workspace
    assert!(!context.contains("Secret data"));
    assert!(!context.contains("/secret.txt"));
}

#[tokio::test]
async fn test_build_context_empty_chat() {
    let test_app = TestApp::new("test_build_context_empty_chat").await;
    let mut conn = test_app.get_connection().await;

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file
    let chat_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "empty_chat".to_string(),
        slug: None,
        path: None,
        is_virtual: Some(true),
        is_remote: None,
        permission: None,
        file_type: FileType::Chat,
        content: serde_json::json!({}),
        app_data: None,
    };
    let chat = create_file_with_content(&mut conn, chat_request)
        .await
        .expect("Failed to create chat file");

    let context = ChatService::build_context(&mut conn, workspace.id, chat.file.id)
        .await
        .expect("Failed to build context");

    // Should contain only persona (no history or attachments)
    assert!(context.contains("BuildScale AI"));
    assert!(!context.contains("Conversation History:"));
}

#[tokio::test]
async fn test_build_context_token_limit_optimization() {
    let test_app = TestApp::new("test_build_context_token_limit_optimization").await;
    let mut conn = test_app.get_connection().await;

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file
    let chat_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "large_chat".to_string(),
        slug: None,
        path: None,
        is_virtual: Some(true),
        is_remote: None,
        permission: None,
        file_type: FileType::Chat,
        content: serde_json::json!({}),
        app_data: None,
    };
    let chat = create_file_with_content(&mut conn, chat_request)
        .await
        .expect("Failed to create chat file");

    // Add many messages to exceed token limit
    for i in 0..100 {
        chat::insert_chat_message(
            &mut conn,
            buildscale::models::chat::NewChatMessage {
                file_id: chat.file.id,
                workspace_id: workspace.id,
                role: ChatMessageRole::User,
                content: format!(
                    "This is message number {} with some content to increase token count",
                    i
                ),
                metadata: sqlx::types::Json(ChatMessageMetadata::default()),
            },
        )
        .await
        .expect("Failed to insert message");
    }

    // Create several large file attachments
    for i in 0..10 {
        let file_request = CreateFileRequest {
            workspace_id: workspace.id,
            parent_id: None,
            author_id: user.id,
            name: format!("file{}.txt", i),
            slug: None,
            path: Some(format!("/file{}.txt", i)),
            is_virtual: None,
            is_remote: None,
            permission: None,
            file_type: FileType::Document,
            content: serde_json::json!({"text": "A".repeat(1000)}),
            app_data: None,
        };
        let file = create_file_with_content(&mut conn, file_request)
            .await
            .expect("Failed to create file");

        chat::insert_chat_message(
            &mut conn,
            buildscale::models::chat::NewChatMessage {
                file_id: chat.file.id,
                workspace_id: workspace.id,
                role: ChatMessageRole::User,
                content: format!("Please read file {}", i),
                metadata: sqlx::types::Json(ChatMessageMetadata {
                    attachments: vec![ChatAttachment::File {
                        file_id: file.file.id,
                        version_id: None,
                    }],
                    tool_calls: None,
                    usage: None,
                }),
            },
        )
        .await
        .expect("Failed to insert message with attachment");
    }

    let context = ChatService::build_context(&mut conn, workspace.id, chat.file.id)
        .await
        .expect("Failed to build context");

    // Context should be optimized to fit within DEFAULT_CONTEXT_TOKEN_LIMIT
    // Rough estimation: 4000 tokens * 4 chars/token = 16000 chars
    // Allow some margin for estimation errors
    assert!(
        context.len() < DEFAULT_CONTEXT_TOKEN_LIMIT * 5,
        "Context should be optimized to fit within token limit, but got {} chars",
        context.len()
    );

    // Persona should always be present (essential)
    assert!(context.contains("BuildScale AI"));
}

#[tokio::test]
async fn test_build_context_fragment_ordering() {
    let test_app = TestApp::new("test_build_context_fragment_ordering").await;
    let mut conn = test_app.get_connection().await;

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
    let chat = create_file_with_content(&mut conn, chat_request)
        .await
        .expect("Failed to create chat file");

    // Add message first
    chat::insert_chat_message(
        &mut conn,
        buildscale::models::chat::NewChatMessage {
            file_id: chat.file.id,
            workspace_id: workspace.id,
            role: ChatMessageRole::User,
            content: "Test message".to_string(),
            metadata: sqlx::types::Json(ChatMessageMetadata::default()),
        },
    )
    .await
    .expect("Failed to insert message");

    // Then add file attachment
    let file_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "test.txt".to_string(),
        slug: None,
        path: Some("/test.txt".to_string()),
        is_virtual: None,
        is_remote: None,
        permission: None,
        file_type: FileType::Document,
        content: serde_json::json!({"text": "File content"}),
        app_data: None,
    };
    let file = create_file_with_content(&mut conn, file_request)
        .await
        .expect("Failed to create file");

    chat::insert_chat_message(
        &mut conn,
        buildscale::models::chat::NewChatMessage {
            file_id: chat.file.id,
            workspace_id: workspace.id,
            role: ChatMessageRole::User,
            content: "Read this".to_string(),
            metadata: sqlx::types::Json(ChatMessageMetadata {
                attachments: vec![ChatAttachment::File {
                    file_id: file.file.id,
                    version_id: None,
                }],
                tool_calls: None,
                usage: None,
            }),
        },
    )
    .await
    .expect("Failed to insert message with attachment");

    let context = ChatService::build_context(&mut conn, workspace.id, chat.file.id)
        .await
        .expect("Failed to build context");

    // Verify ordering: persona comes first, files come before history
    let persona_pos = context.find("BuildScale AI").unwrap();
    let file_pos = context.find("<file_context>").unwrap();
    let history_pos = context.find("Conversation History:").unwrap();

    assert!(persona_pos < file_pos, "Persona should come before files");
    assert!(file_pos < history_pos, "Files should come before history");
}
