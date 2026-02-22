use crate::common::database::TestApp;
use buildscale::load_config;
use buildscale::models::chat::{ChatMessageRole, NewChatMessage};
use buildscale::models::files::FileType;
use buildscale::models::requests::CreateFileRequest;
use buildscale::queries::chat;
use buildscale::services::chat::actor::{ChatActor, ChatActorArgs};
use buildscale::services::chat::rig_engine::RigService;
use buildscale::services::chat::registry::{AgentRegistry, AgentCommand};
use buildscale::services::files::create_file_with_content;
use buildscale::services::storage::FileStorageService;
use std::sync::Arc;
use tokio::time::{Duration};

#[tokio::test]
async fn test_chat_actor_inactivity_timeout() {
    let app = TestApp::new("test_chat_actor_inactivity_timeout").await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file with proper app_data (model and mode)
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
            "model": "gpt-4o",
            "mode": "chat"
        })),
    };
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
        .await
        .expect("Failed to create chat file");

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    // Insert a chat message
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
    let registry = Arc::new(AgentRegistry::new());
    let (event_tx, _) = tokio::sync::broadcast::channel(100);
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    // Use a short timeout for testing (200ms)
    let timeout = Duration::from_millis(200);
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
        inactivity_timeout: timeout,
    });

    // Actor should be alive initially
    assert!(!handle.command_tx.is_closed(), "Actor should be alive initially");

    // Wait for slightly more than the timeout
    tokio::time::sleep(timeout + Duration::from_millis(100)).await;

    // Now the actor should have shut down and closed the channel
    assert!(handle.command_tx.is_closed(), "Actor should have shut down after inactivity timeout");
}

#[tokio::test]
async fn test_agent_registry_cleanup() {
    let app = TestApp::new("test_agent_registry_cleanup").await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file with proper app_data (model and mode)
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
            "model": "gpt-4o",
            "mode": "chat"
        })),
    };
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
        .await
        .expect("Failed to create chat file");

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    // Insert a chat message
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
    let registry = Arc::new(AgentRegistry::new());
    let event_tx = registry.get_or_create_bus(chat_id).await;
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    // Use a short timeout for testing (200ms)
    let timeout = Duration::from_millis(200);
    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        user_id,
        pool: app.test_db.pool.clone(),
        rig_service,
        storage,
        registry: registry.clone(),
        default_persona: "test persona".to_string(),
        default_context_token_limit: 1000,
        event_tx,
        inactivity_timeout: timeout,
    });

    registry.register(chat_id, handle.clone()).await;

    // Registry should return the handle initially
    assert!(registry.get_handle(&chat_id).await.is_some(), "Registry should return active handle");

    // Wait for slightly more than the timeout
    tokio::time::sleep(timeout + Duration::from_millis(100)).await;

    // Registry should detect the closed channel, remove it, and return None
    assert!(registry.get_handle(&chat_id).await.is_none(), "Registry should remove and not return timed-out handle");
}

#[tokio::test]
async fn test_chat_actor_timeout_reset() {
    let app = TestApp::new("test_chat_actor_timeout_reset").await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file with proper app_data (model and mode)
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
            "model": "gpt-4o",
            "mode": "chat"
        })),
    };
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
        .await
        .expect("Failed to create chat file");

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    // Insert a chat message
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
    let registry = Arc::new(AgentRegistry::new());
    let (event_tx, _) = tokio::sync::broadcast::channel(100);
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    // Use a 500ms timeout
    let timeout = Duration::from_millis(500);
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
        inactivity_timeout: timeout,
    });

    // Wait for 300ms (more than half)
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!handle.command_tx.is_closed());

    // Send a Ping command to reset the timer
    let _ = handle.command_tx.send(AgentCommand::Ping).await;

    // Wait another 300ms.
    // Total time since start = 600ms (> 500ms timeout).
    // But since we reset at 300ms, it should still be alive.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!handle.command_tx.is_closed(), "Actor should still be alive after reset");

    // Now wait for the new timeout to expire (another 300ms)
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(handle.command_tx.is_closed(), "Actor should shut down after second timeout expires");
}

// ============================================================================
// SESSION REUSE TESTS
// ============================================================================

#[tokio::test]
async fn test_chat_actor_creates_new_session_when_none_exists() {
    let app = TestApp::new("test_chat_actor_creates_new_session").await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file with proper app_data (model and mode)
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
            "model": "gpt-4o",
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
    let (event_tx, _) = tokio::sync::broadcast::channel(100);
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    // Spawn the actor - should create a new session
    let _handle = ChatActor::spawn(ChatActorArgs {
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
        inactivity_timeout: Duration::from_millis(500),
    });

    // Give the actor time to create the session
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify a session was created in the database
    let session = buildscale::queries::agent_sessions::get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Failed to query session");

    assert!(session.is_some(), "Session should have been created");
    let session = session.unwrap();
    assert_eq!(session.status, buildscale::models::agent_session::SessionStatus::Idle);
    assert_eq!(session.chat_id, chat_id);
}

#[tokio::test]
async fn test_chat_actor_reuses_terminal_session() {
    let app = TestApp::new("test_chat_actor_reuses_terminal_session").await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file with proper app_data (model and mode)
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
            "model": "gpt-4o",
            "mode": "chat"
        })),
    };
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
        .await
        .expect("Failed to create chat file");

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    // Create a completed session manually
    let new_session = buildscale::models::agent_session::NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: buildscale::models::agent_session::AgentType::Assistant,
        model: "gpt-4o".to_string(),
        mode: "chat".to_string(),
    };
    let session = buildscale::queries::agent_sessions::create_session(&mut conn, new_session)
        .await
        .expect("Failed to create session");

    // Mark it as completed
    let completed_session = buildscale::queries::agent_sessions::update_session_status(
        &mut conn,
        session.id,
        buildscale::models::agent_session::SessionStatus::Completed,
        None,
    )
    .await
    .expect("Failed to update session status");

    assert_eq!(
        completed_session.status,
        buildscale::models::agent_session::SessionStatus::Completed
    );

    let rig_service = Arc::new(RigService::dummy());
    let registry = Arc::new(AgentRegistry::new());
    let (event_tx, _) = tokio::sync::broadcast::channel(100);
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    // Spawn the actor - should reuse the completed session
    let _handle = ChatActor::spawn(ChatActorArgs {
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
        inactivity_timeout: Duration::from_millis(500),
    });

    // Give the actor time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify the session was reused (same ID, now idle)
    let updated_session = buildscale::queries::agent_sessions::get_session_by_id(&mut conn, session.id)
        .await
        .expect("Failed to query session")
        .expect("Session should exist");

    assert_eq!(updated_session.id, session.id, "Session ID should be the same");
    assert_eq!(
        updated_session.status,
        buildscale::models::agent_session::SessionStatus::Idle,
        "Session should be back to idle"
    );
    assert!(updated_session.completed_at.is_none(), "completed_at should be cleared");
}

#[tokio::test]
async fn test_chat_actor_errors_with_active_session() {
    let app = TestApp::new("test_chat_actor_errors_with_active_session").await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = app.create_test_workspace_with_user().await.unwrap();

    // Create a chat file with proper app_data (model and mode)
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
            "model": "gpt-4o",
            "mode": "chat"
        })),
    };
    let chat = create_file_with_content(&mut conn, &storage, chat_request)
        .await
        .expect("Failed to create chat file");

    let chat_id = chat.file.id;
    let workspace_id = workspace.id;
    let user_id = user.id;

    // Create an active (idle) session manually
    let new_session = buildscale::models::agent_session::NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: buildscale::models::agent_session::AgentType::Assistant,
        model: "gpt-4o".to_string(),
        mode: "chat".to_string(),
    };
    let _session = buildscale::queries::agent_sessions::create_session(&mut conn, new_session)
        .await
        .expect("Failed to create session");

    let rig_service = Arc::new(RigService::dummy());
    let registry = Arc::new(AgentRegistry::new());
    let (event_tx, _event_rx) = tokio::sync::broadcast::channel(100);
    let storage = Arc::new(FileStorageService::new(&load_config().unwrap().storage.base_path));

    // Spawn the actor - should fail due to active session
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
        inactivity_timeout: Duration::from_millis(500),
    });

    // Give the actor time to attempt creation and fail
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Actor should have shut down immediately due to session conflict
    assert!(handle.command_tx.is_closed(), "Actor should have shut down due to active session conflict");

    // The original session should still be in the database (unchanged)
    let original_session = buildscale::queries::agent_sessions::get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Failed to query session")
        .expect("Session should still exist");

    assert_eq!(
        original_session.status,
        buildscale::models::agent_session::SessionStatus::Idle,
        "Original session should still be idle"
    );
}
