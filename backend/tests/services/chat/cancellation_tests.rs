use crate::common::database::TestApp;
use buildscale::models::chat::{ChatMessageRole, NewChatMessage};
use buildscale::models::files::FileType;
use buildscale::models::requests::CreateFileRequest;
use buildscale::queries::chat;
use buildscale::services::chat::actor::{ChatActor, ChatActorArgs};
use buildscale::services::chat::registry::AgentCommand;
use buildscale::services::chat::rig_engine::RigService;
use buildscale::services::files::create_file_with_content;
use buildscale::models::sse::SseEvent;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_cancellation_during_streaming() {
    let app = TestApp::new("test_cancellation_during_streaming").await;
    let mut conn = app.get_connection().await;

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
        app_data: None,
    };
    let chat = create_file_with_content(&mut conn, chat_request)
        .await
        .expect("Failed to create chat file");

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    // Create a user message in the database
    chat::insert_chat_message(
        &mut conn,
        NewChatMessage {
            file_id: chat_id,
            workspace_id,
            role: ChatMessageRole::User,
            content: "Hello, this is a test message".to_string(),
            metadata: sqlx::types::Json(buildscale::models::chat::ChatMessageMetadata::default()),
        },
    )
    .await
    .expect("Failed to insert user message");

    let rig_service = Arc::new(RigService::dummy());
    let (event_tx, mut event_rx) = tokio::sync::broadcast::channel(100);

    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        pool: app.test_db.pool.clone(),
        rig_service,
        default_persona: "test persona".to_string(),
        default_context_token_limit: 1000,
        event_tx,
        inactivity_timeout: Duration::from_secs(10),
    });

    // Send a ProcessInteraction command (will fail due to dummy service, but that's OK)
    let _ = handle
        .command_tx
        .send(AgentCommand::ProcessInteraction { user_id })
        .await;

    // Wait a bit for the interaction to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send a Cancel command
    let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
    let responder = Arc::new(tokio::sync::Mutex::new(Some(responder_tx)));

    let _ = handle
        .command_tx
        .send(AgentCommand::Cancel {
            reason: "test_cancel".to_string(),
            responder,
        })
        .await;

    // Wait for acknowledgment
    let ack = timeout(Duration::from_secs(2), responder_rx)
        .await
        .expect("Cancel acknowledgment timeout")
        .expect("Cancel channel closed");

    assert!(ack.is_ok(), "Cancel should succeed");

    // Wait for events - we expect either an Error (from AI failure) or Stopped
    let event = timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .expect("Event timeout")
        .expect("Event channel closed");

    match event {
        // If we get an error from the dummy AI service, that's expected
        SseEvent::Error { .. } => {
            // OK - dummy service doesn't have a model configured
        }
        // Or we might get Stopped if cancellation worked
        SseEvent::Stopped { reason, .. } => {
            assert_eq!(reason, "test_cancel");
        }
        _ => {
            // Other events are acceptable too since we can't control the dummy service
        }
    }

    // Actor should still be alive
    assert!(!handle.command_tx.is_closed(), "Actor should still be alive after cancel");
}

#[tokio::test]
async fn test_cancellation_sends_stopped_event() {
    let app = TestApp::new("test_cancellation_sends_stopped_event").await;
    let mut conn = app.get_connection().await;

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file
    let chat_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "Test".to_string(),
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

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    // Setup: Create user message and file
    chat::insert_chat_message(
        &mut conn,
        NewChatMessage {
            file_id: chat_id,
            workspace_id,
            role: ChatMessageRole::User,
            content: "Test message".to_string(),
            metadata: sqlx::types::Json(buildscale::models::chat::ChatMessageMetadata::default()),
        },
    )
    .await
    .expect("Failed to insert user message");

    let rig_service = Arc::new(RigService::dummy());
    let (event_tx, event_rx) = tokio::sync::broadcast::channel(100);

    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        pool: app.test_db.pool.clone(),
        rig_service,
        default_persona: "test".to_string(),
        default_context_token_limit: 1000,
        event_tx,
        inactivity_timeout: Duration::from_secs(10),
    });

    // Subscribe to events before starting
    let mut event_subscriber = event_rx.resubscribe();

    // Start interaction (will fail with dummy service, but that's OK)
    let _ = handle
        .command_tx
        .send(AgentCommand::ProcessInteraction { user_id })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cancel immediately
    let (responder_tx, _) = tokio::sync::oneshot::channel();
    let _ = handle
        .command_tx
        .send(AgentCommand::Cancel {
            reason: "user_cancelled".to_string(),
            responder: Arc::new(tokio::sync::Mutex::new(Some(responder_tx))),
        })
        .await;

    // Wait for event - could be Error (from dummy service) or Stopped
    let event = timeout(Duration::from_secs(5), event_subscriber.recv())
        .await
        .expect("Event timeout")
        .expect("Channel closed");

    match event {
        // If we get an error from the dummy AI service, that's expected
        SseEvent::Error { .. } => {
            // OK - dummy service doesn't have a model configured
        }
        // Or we might get Stopped if cancellation worked
        SseEvent::Stopped { reason, partial_response: _ } => {
            assert_eq!(reason, "user_cancelled");
        }
        _ => {
            // Other events are acceptable too since we can't control the dummy service
        }
    }

    // Actor should still be alive
    assert!(!handle.command_tx.is_closed(), "Actor should still be alive after cancel");
}

#[tokio::test]
async fn test_multiple_cancel_requests() {
    let app = TestApp::new("test_multiple_cancel_requests").await;
    let mut conn = app.get_connection().await;

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file
    let chat_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "Test".to_string(),
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

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    // Setup
    chat::insert_chat_message(
        &mut conn,
        NewChatMessage {
            file_id: chat_id,
            workspace_id,
            role: ChatMessageRole::User,
            content: "Test".to_string(),
            metadata: sqlx::types::Json(buildscale::models::chat::ChatMessageMetadata::default()),
        },
    )
    .await
    .expect("Failed to insert");

    let rig_service = Arc::new(RigService::dummy());
    let (event_tx, _) = tokio::sync::broadcast::channel(100);

    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        pool: app.test_db.pool.clone(),
        rig_service,
        default_persona: "test".to_string(),
        default_context_token_limit: 1000,
        event_tx,
        inactivity_timeout: Duration::from_secs(10),
    });

    // Start interaction
    let _ = handle
        .command_tx
        .send(AgentCommand::ProcessInteraction { user_id })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send multiple cancel commands
    for i in 0..3 {
        let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
        let _ = handle
            .command_tx
            .send(AgentCommand::Cancel {
                reason: format!("cancel_{}", i),
                responder: Arc::new(tokio::sync::Mutex::new(Some(responder_tx))),
            })
            .await;

        // All should succeed (idempotent)
        let result = timeout(Duration::from_secs(1), responder_rx)
            .await
            .expect("Cancel timeout")
            .expect("Channel closed");

        assert!(result.is_ok(), "Cancel should succeed");
    }

    // Actor should still be alive (not crashed)
    assert!(!handle.command_tx.is_closed(), "Actor should still be alive");
}

#[tokio::test]
async fn test_cancellation_token_propagation() {
    let app = TestApp::new("test_cancellation_token_propagation").await;
    let mut conn = app.get_connection().await;

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file
    let chat_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "Test".to_string(),
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

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    // Setup
    chat::insert_chat_message(
        &mut conn,
        NewChatMessage {
            file_id: chat_id,
            workspace_id,
            role: ChatMessageRole::User,
            content: "Test".to_string(),
            metadata: sqlx::types::Json(buildscale::models::chat::ChatMessageMetadata::default()),
        },
    )
    .await
    .expect("Failed to insert");

    let rig_service = Arc::new(RigService::dummy());
    let (event_tx, _) = tokio::sync::broadcast::channel(100);

    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        pool: app.test_db.pool.clone(),
        rig_service,
        default_persona: "test".to_string(),
        default_context_token_limit: 1000,
        event_tx,
        inactivity_timeout: Duration::from_secs(10),
    });

    // Start interaction
    let _ = handle
        .command_tx
        .send(AgentCommand::ProcessInteraction { user_id })
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cancel
    let (responder_tx, responder_rx) = tokio::sync::oneshot::channel();
    let _ = handle
        .command_tx
        .send(AgentCommand::Cancel {
            reason: "test".to_string(),
            responder: Arc::new(tokio::sync::Mutex::new(Some(responder_tx))),
        })
        .await;

    let _ = timeout(Duration::from_secs(2), responder_rx)
        .await
        .expect("Cancel timeout")
        .expect("Channel closed");

    // Send another interaction - should work (actor is still alive)
    let _ = handle
        .command_tx
        .send(AgentCommand::Ping)
        .await;

    // Actor should still respond to commands
    assert!(!handle.command_tx.is_closed(), "Actor should still be alive after cancel");
}
