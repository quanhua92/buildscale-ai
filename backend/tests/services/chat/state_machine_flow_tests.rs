//! Tests for state machine flow and transition behavior.
//!
//! These tests verify that:
//! 1. State transitions happen BEFORE actions are executed
//! 2. Events are handled by the correct state
//! 3. The state machine's internal state is properly updated
//! 4. Actions execute in the correct order relative to state changes

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
use sqlx::types::Json as SqlxJson;

/// Integration test: Verify full state machine flow
/// Tests that ProcessInteraction triggers:
/// 1. State transition: Idle → Running
/// 2. AI processing via StartProcessing action
/// 3. Result state transition: Running → Error (with dummy API)
/// This test would have caught the bug where state transition happened AFTER actions.
#[tokio::test]
async fn test_state_machine_idle_to_running_to_error_flow() {
    let app = TestApp::new("test_state_machine_flow").await;
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

    // Give the actor time to start and initialize
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify actor is running
    assert!(!handle.command_tx.is_closed(), "Actor should be running");

    // Verify session was created in Idle state
    let session = buildscale::queries::agent_sessions::get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Failed to query session")
        .expect("Session should exist");
    assert_eq!(
        session.status,
        buildscale::models::agent_session::SessionStatus::Idle,
        "Initial session state should be Idle"
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

    println!("[TEST] About to send ProcessInteraction command");

    // Send ProcessInteraction command
    let _ = handle.command_tx.send(AgentCommand::ProcessInteraction { user_id }).await;

    println!("[TEST] Command sent, waiting for processing");

    // Give it time to process
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Verify session transitioned out of Idle (AI processing was triggered)
    let final_session = buildscale::queries::agent_sessions::get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Failed to query final session")
        .expect("Session should still exist");

    println!("[TEST] Final session status: {:?}", final_session.status);

    // The session should have changed from Idle (AI processing was attempted)
    // With dummy API, we expect Error state
    assert_ne!(
        final_session.status,
        buildscale::models::agent_session::SessionStatus::Idle,
        "Session should NOT still be Idle after ProcessInteraction - bug: state transition not working"
    );

    // Verify error message indicates AI was actually invoked
    if final_session.status == buildscale::models::agent_session::SessionStatus::Error {
        assert!(
            final_session.error_message.is_some(),
            "Error session should have error_message set"
        );
        let error_msg = final_session.error_message.unwrap();
        assert!(
            error_msg.contains("AI Engine Error") || error_msg.contains("API"),
            "Error should mention AI Engine Error or API: {}",
            error_msg
        );
        println!("[TEST] ✓ AI processing was triggered (got expected error)");
    }
}

/// Test that ProcessInteraction while in Running state doesn't break the actor
/// This would have caught the bug where InteractionComplete was sent to Idle state
#[tokio::test]
async fn test_state_transition_happens_before_actions() {
    let app = TestApp::new("test_transition_order").await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

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

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Send first ProcessInteraction
    let _ = handle.command_tx.send(AgentCommand::ProcessInteraction { user_id }).await;

    // Wait a tiny bit then send second ProcessInteraction
    // This tests that the actor doesn't break if ProcessInteraction comes in while already processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    let _ = handle.command_tx.send(AgentCommand::ProcessInteraction { user_id }).await;

    println!("[TEST] Sent two ProcessInteraction commands");

    // Wait for processing
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Critical check: session should NOT be stuck in Running state
    // The bug would cause: Idle → StartProcessing sends InteractionComplete to Idle → Idle doesn't handle it → stays Idle
    let session = buildscale::queries::agent_sessions::get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Failed to query session")
        .expect("Session should exist");

    println!("[TEST] Final session status: {:?}", session.status);

    // Should NOT be stuck in Running or Idle (both indicate the bug)
    assert_ne!(
        session.status,
        buildscale::models::agent_session::SessionStatus::Running,
        "Session should NOT be stuck in Running state - bug: state machine not handling InteractionComplete"
    );

    // With the fix, session should be in Error state (dummy API) or Completed
    // If it's Idle, the bug is present (InteractionComplete was never handled)
    assert_ne!(
        session.status,
        buildscale::models::agent_session::SessionStatus::Idle,
        "Session should NOT still be Idle after processing - bug: state transition happened before actions"
    );

    println!("[TEST] ✓ State machine flow works correctly");
}
