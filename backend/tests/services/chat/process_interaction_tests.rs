//! Tests for the ProcessInteraction flow in ChatActor.
//!
//! These tests verify that when a ProcessInteraction command is sent to the ChatActor:
//! 1. The state transitions happen correctly (Idle → Running → Idle)
//! 2. The actual AI processing is triggered (not just state changes)
//! 3. The response is generated and sent via SSE
//! 4. The session status is updated correctly throughout

use crate::common::database::TestApp;
use buildscale::load_config;
use buildscale::models::chat::{ChatMessageRole, NewChatMessage, DEFAULT_CHAT_MODEL};
use buildscale::models::files::FileType;
use buildscale::models::requests::CreateFileRequest;
use buildscale::queries::chat;
use buildscale::services::chat::actor::{ChatActor, ChatActorArgs};
use buildscale::services::chat::rig_engine::RigService;
use buildscale::services::chat::registry::{AgentCommand, AgentRegistry};
use buildscale::services::files::create_file_with_content;
use buildscale::services::storage::FileStorageService;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use sqlx::types::Json as SqlxJson;

/// Helper struct to track SSE events received during a test
struct TestSseReceiver {
    receiver: broadcast::Receiver<buildscale::models::sse::SseEvent>,
    events: Vec<buildscale::models::sse::SseEvent>,
}

impl TestSseReceiver {
    fn new(receiver: broadcast::Receiver<buildscale::models::sse::SseEvent>) -> Self {
        Self {
            receiver,
            events: Vec::new(),
        }
    }

    /// Collect all events for a specified duration
    async fn collect_events(&mut self, duration: Duration) {
        use tokio::sync::broadcast::error::RecvError;
        let start = std::time::Instant::now();
        while start.elapsed() < duration {
            match tokio::time::timeout(Duration::from_millis(100), self.receiver.recv()).await {
                Ok(Ok(event)) => {
                    self.events.push(event);
                }
                Ok(Err(RecvError::Lagged(_))) => {
                    // Lagged - skip missed messages and continue
                    continue;
                }
                Ok(Err(RecvError::Closed)) => {
                    // Channel closed
                    break;
                }
                Err(_) => {
                    // Timeout, check if we should continue
                    if start.elapsed() >= duration {
                        break;
                    }
                }
            }
        }
    }

    /// Get the count of Chunk events
    fn chunk_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, buildscale::models::sse::SseEvent::Chunk { .. }))
            .count()
    }

    /// Check if there was an error event
    fn has_error(&self) -> bool {
        self.events.iter().any(|e| matches!(e, buildscale::models::sse::SseEvent::Error { .. }))
    }

    /// Check if there was a Done event
    fn has_done(&self) -> bool {
        self.events.iter().any(|e| matches!(e, buildscale::models::sse::SseEvent::Done { .. }))
    }
}

#[tokio::test]
async fn test_process_interaction_triggers_ai_processing() {
    let app = TestApp::new("test_process_interaction_triggers_ai_processing").await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file
    let chat_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "Test Chat".to_string(),
        slug: None,
        path: None,
        is_virtual: Some(true),
        is_remote: None,
        permission: None,
        file_type: FileType::Chat,
        content: serde_json::json!({}),
        app_data: Some(serde_json::json!({
            "model": DEFAULT_CHAT_MODEL,
            "mode": "chat"
        })),
    };
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
        .await
        .expect("Failed to create chat file");

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    let rig_service = Arc::new(RigService::dummy());
    let registry = Arc::new(AgentRegistry::new());
    let (event_tx, _event_rx) = tokio::sync::broadcast::channel(100);
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    // Spawn the actor
    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        user_id,
        pool: app.test_db.pool.clone(),
        rig_service,
        storage,
        registry,
        default_persona: "test persona".to_string(),
        default_context_token_limit: 1000,
        event_tx,
        inactivity_timeout: Duration::from_secs(10),
    });

    // Give the actor time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify the actor is running
    assert!(!handle.command_tx.is_closed(), "Actor should be running");

    // Verify session was created
    let session = buildscale::queries::agent_sessions::get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Failed to query session");
    assert!(session.is_some(), "Session should exist");
    let session = session.unwrap();
    assert_eq!(
        session.status,
        buildscale::models::agent_session::SessionStatus::Idle
    );

    // Add a user message
    let new_message = NewChatMessage {
        file_id: chat_id,
        workspace_id,
        role: ChatMessageRole::User,
        content: "Hello, test message".to_string(),
        metadata: SqlxJson(buildscale::models::chat::ChatMessageMetadata::default()),
    };
    chat::insert_chat_message(&mut conn, new_message)
        .await
        .expect("Failed to create message");

    // Send ProcessInteraction command
    println!("[TEST] About to send ProcessInteraction command");
    println!("[TEST] Actor channel closed: {}", handle.command_tx.is_closed());
    handle.command_tx.send(AgentCommand::ProcessInteraction { user_id }).await;
    println!("[TEST] Command sent successfully");

    // Wait for processing to complete
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Check session status to see what happened
    let session_after = buildscale::queries::agent_sessions::get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Failed to query session")
        .expect("Session should exist");

    println!("[TEST] Session status after processing: {:?}", session_after.status);
    println!("[TEST] Session error_message: {:?}", session_after.error_message);

    // The key assertion: AI processing should have been triggered
    // We can verify this by checking that a response message was created
    let messages = chat::get_messages_by_file_id(&mut conn, workspace_id, chat_id)
        .await
        .expect("Failed to get messages");

    // Should have at least 2 messages: user message + AI response
    assert!(
        messages.len() >= 2,
        "Should have at least 2 messages (user + AI response), got {}.
         THIS TEST FAILS BECAUSE THE STATE HANDLER IS EATING THE ProcessInteraction EVENT
         WITHOUT TRIGGERING THE ACTUAL AI PROCESSING.",
        messages.len()
    );

    // Last message should be from AI
    let last_message = messages.last().expect("Should have messages");
    assert_eq!(
        last_message.role,
        ChatMessageRole::Assistant,
        "Last message should be from Assistant"
    );

    // Verify the content is not empty (AI actually responded)
    assert!(
        !last_message.content.is_empty(),
        "AI response should not be empty"
    );

    // Verify session eventually returns to Idle (processing completed)
    tokio::time::sleep(Duration::from_secs(2)).await;
    let final_session = buildscale::queries::agent_sessions::get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Failed to query session")
        .expect("Session should exist");

    // Session should be back to Idle after successful processing
    assert_eq!(
        final_session.status,
        buildscale::models::agent_session::SessionStatus::Idle,
        "Session should be Idle after processing completes, got {:?}",
        final_session.status
    );
}

#[tokio::test]
async fn test_process_interaction_creates_response_message() {
    let app = TestApp::new("test_process_interaction_creates_response").await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file
    let chat_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "Test Chat".to_string(),
        slug: None,
        path: None,
        is_virtual: Some(true),
        is_remote: None,
        permission: None,
        file_type: FileType::Chat,
        content: serde_json::json!({}),
        app_data: Some(serde_json::json!({
            "model": DEFAULT_CHAT_MODEL,
            "mode": "chat"
        })),
    };
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
        .await
        .expect("Failed to create chat file");

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    let rig_service = Arc::new(RigService::dummy());
    let registry = Arc::new(AgentRegistry::new());
    let (event_tx, event_rx) = tokio::sync::broadcast::channel(100);
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    // Spawn the actor
    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        user_id,
        pool: app.test_db.pool.clone(),
        rig_service,
        storage,
        registry,
        default_persona: "You are a helpful assistant who responds briefly.".to_string(),
        default_context_token_limit: 1000,
        event_tx,
        inactivity_timeout: Duration::from_secs(10),
    });

    // Give the actor time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Collect SSE events
    let mut sse_receiver = TestSseReceiver::new(event_rx);

    // Add a user message
    let user_content = "Say 'hello world'";
    let new_message = NewChatMessage {
        file_id: chat_id,
        workspace_id,
        role: ChatMessageRole::User,
        content: user_content.to_string(),
        metadata: SqlxJson(buildscale::models::chat::ChatMessageMetadata::default()),
    };
    chat::insert_chat_message(&mut conn, new_message)
        .await
        .expect("Failed to create message");

    // Send ProcessInteraction command
    let _ = handle.command_tx.send(AgentCommand::ProcessInteraction { user_id });

    // Collect events during processing
    sse_receiver.collect_events(Duration::from_secs(5)).await;

    // Verify we got Chunk events (showing AI processing happened)
    assert!(
        sse_receiver.chunk_count() >= 1,
        "Should have at least 1 Chunk event (AI processing happened), got {}.
         THIS TEST FAILS BECAUSE THE STATE HANDLER IS EATING THE ProcessInteraction EVENT
         WITHOUT TRIGGERING THE ACTUAL AI PROCESSING.",
        sse_receiver.chunk_count()
    );

    // Verify we got a Done event
    assert!(
        sse_receiver.has_done(),
        "Should have a Done event after processing completes"
    );

    // Verify no errors occurred
    assert!(
        !sse_receiver.has_error(),
        "Should not have any error events. Events: {:?}",
        sse_receiver.events
    );

    // Verify response message was created
    let messages = chat::get_messages_by_file_id(&mut conn, workspace_id, chat_id)
        .await
        .expect("Failed to get messages");

    // Should have exactly 2 messages: user + AI response
    assert_eq!(messages.len(), 2, "Should have exactly 2 messages");

    // Verify the AI response
    let ai_message = &messages[1];
    assert_eq!(
        ai_message.role,
        ChatMessageRole::Assistant,
        "Second message should be from Assistant"
    );

    // Verify content is not empty
    assert!(
        !ai_message.content.trim().is_empty(),
        "AI response should not be empty"
    );

    // The response should contain "hello" (since we asked for it)
    let response_lower = ai_message.content.to_lowercase();
    assert!(
        response_lower.contains("hello") || response_lower.contains("hi"),
        "Response should contain greeting. Got: {}",
        ai_message.content
    );
}
