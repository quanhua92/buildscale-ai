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
use buildscale::services::chat::{AttachmentKey, ChatService, DEFAULT_CONTEXT_TOKEN_LIMIT};
use buildscale::services::files::create_file_with_content;
use buildscale::services::storage::FileStorageService;
use buildscale::load_config;
use crate::common::database::TestApp;

#[tokio::test]
async fn test_build_context_with_persona() {
    let test_app = TestApp::new("test_build_context_with_persona").await;
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

    let context = ChatService::build_context(&mut conn, &storage, workspace.id, chat.file.id, "You are BuildScale AI, a professional software engineering assistant.", 4000)
        .await
        .expect("Failed to build context");

    // Should contain system persona
    assert!(context.persona.contains("BuildScale AI"));
    assert!(context.persona.contains("professional software engineering assistant"));
}

#[tokio::test]
async fn test_build_context_with_history() {
    let test_app = TestApp::new("test_build_context_with_history").await;
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

    let context = ChatService::build_context(&mut conn, &storage, workspace.id, chat.file.id, "You are BuildScale AI, a professional software engineering assistant.", 4000)
        .await
        .expect("Failed to build context");

    // Should contain conversation history (excluding last/current message)
    assert!(!context.history.is_empty());
    assert_eq!(context.history.len(), 1, "History should exclude the last message which is the current prompt");
    assert_eq!(context.history[0].content, "Hello, how are you?");
}

#[tokio::test]
async fn test_build_context_with_file_attachments() {
    let test_app = TestApp::new("test_build_context_with_file_attachments").await;
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
    let file = create_file_with_content(&mut conn, &storage, file_request)
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
                model: None,
                response_id: None,
            }),
        },
    )
    .await
    .expect("Failed to insert message with attachment");

    let context = ChatService::build_context(&mut conn, &storage, workspace.id, chat.file.id, "You are BuildScale AI, a professional software engineering assistant.", 4000)
        .await
        .expect("Failed to build context");

    // Should contain file attachments in ContextManager
    assert!(!context.attachment_manager.map.is_empty());
    assert_eq!(context.attachment_manager.map.len(), 1);

    // Extract the file content from the ContextManager
    let file_content = context.attachment_manager.map
        .values()
        .next()
        .expect("Should have one attachment");
    assert!(file_content.content.contains("Hello World"));
}

#[tokio::test]
async fn test_build_context_workspace_isolation() {
    let test_app = TestApp::new("test_build_context_workspace_isolation").await;
    let mut conn = test_app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

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
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
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
    let file = create_file_with_content(&mut conn, &storage, file_request)
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
                model: None,
                response_id: None,
            }),
        },
    )
    .await
    .expect("Failed to insert message with attachment");

    let context = ChatService::build_context(&mut conn, &storage, workspace1.id, chat.file.id, "You are BuildScale AI, a professional software engineering assistant.", 4000)
        .await
        .expect("Failed to build context");

    // Should NOT contain the file from different workspace
    assert!(context.attachment_manager.map.is_empty(), "Attachments should be empty for files from different workspace");
}

#[tokio::test]
async fn test_build_context_empty_chat() {
    let test_app = TestApp::new("test_build_context_empty_chat").await;
    let mut conn = test_app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

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
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
        .await
        .expect("Failed to create chat file");

    let context = ChatService::build_context(&mut conn, &storage, workspace.id, chat.file.id, "You are BuildScale AI, a professional software engineering assistant.", 4000)
        .await
        .expect("Failed to build context");

    // Should contain only persona (no history or attachments)
    assert!(context.persona.contains("BuildScale AI"));
    assert!(context.history.is_empty());
    assert!(context.attachment_manager.map.is_empty());
}

#[tokio::test]
async fn test_build_context_token_limit_optimization() {
    let test_app = TestApp::new("test_build_context_token_limit_optimization").await;
    let mut conn = test_app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

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
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
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
        let file = create_file_with_content(&mut conn, &storage, file_request)
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
                    model: None,
                    response_id: None,
                }),
            },
        )
        .await
        .expect("Failed to insert message with attachment");
    }

    let context = ChatService::build_context(&mut conn, &storage, workspace.id, chat.file.id, "You are BuildScale AI, a professional software engineering assistant.", 4000)
        .await
        .expect("Failed to build context");

    // Context should be optimized to fit within DEFAULT_CONTEXT_TOKEN_LIMIT
    // Check that we have the expected structure
    assert!(!context.persona.is_empty(), "Persona should always be present");
    assert!(context.persona.contains("BuildScale AI"), "Persona should contain AI name");

    // For this test, we just verify the structure is correct
    // Actual token limit optimization is now implemented via ContextManager
    assert!(!context.history.is_empty(), "History should contain messages");
    assert!(!context.attachment_manager.map.is_empty(), "Attachments should contain files");

    // Verify ContextManager has optimized the attachments
    // (It should have pruned some files if over the token limit)
    let total_tokens: usize = context.attachment_manager.map
        .values()
        .map(|v| v.tokens)
        .sum();
    assert!(total_tokens < DEFAULT_CONTEXT_TOKEN_LIMIT * 2,
        "ContextManager should optimize attachments, total tokens: {}", total_tokens);
}

#[tokio::test]
async fn test_build_context_fragment_ordering() {
    let test_app = TestApp::new("test_build_context_fragment_ordering").await;
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
    let file = create_file_with_content(&mut conn, &storage, file_request)
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
                model: None,
                response_id: None,
            }),
        },
    )
    .await
    .expect("Failed to insert message with attachment");

    let context = ChatService::build_context(&mut conn, &storage, workspace.id, chat.file.id, "You are BuildScale AI, a professional software engineering assistant.", 4000)
        .await
        .expect("Failed to build context");

    // Verify structure: we should have persona, history, and attachments
    assert!(!context.persona.is_empty(), "Persona should be present");
    assert!(context.persona.contains("BuildScale AI"), "Persona should contain AI name");

    // History should have at least one message (the "Test message")
    assert!(!context.history.is_empty(), "History should contain messages");

    // Attachments should have the file in ContextManager
    assert!(!context.attachment_manager.map.is_empty(), "Attachments should contain files");
    assert_eq!(context.attachment_manager.map.len(), 1, "Should have one attachment");

    // Verify the AttachmentManager has sorted the fragments
    // WorkspaceFile fragments should be at position POS_WORKSPACE_FILES (2)
    let (key, _value) = context.attachment_manager.map.iter().next().unwrap();
    match key {
        AttachmentKey::WorkspaceFile(_) => {
            // Correct key type
        }
        _ => panic!("Expected WorkspaceFile key"),
    }
}
