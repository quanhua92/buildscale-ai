use crate::common::TestApp;
use buildscale::services::chat::actor::{ChatActor, ChatActorArgs};
use buildscale::services::chat::rig_engine::RigService;
use buildscale::services::chat::registry::{AgentRegistry, AgentCommand};
use uuid::Uuid;
use std::sync::Arc;
use tokio::time::{Duration};

#[tokio::test]
async fn test_chat_actor_inactivity_timeout() {
    let app = TestApp::new().await;
    let chat_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let rig_service = Arc::new(RigService::dummy());
    let (event_tx, _) = tokio::sync::broadcast::channel(100);

    // Use a short timeout for testing (200ms)
    let timeout = Duration::from_millis(200);
    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        pool: app.pool.clone(),
        rig_service,
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
    let app = TestApp::new().await;
    let chat_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let rig_service = Arc::new(RigService::dummy());
    let registry = AgentRegistry::new();
    let event_tx = registry.get_or_create_bus(chat_id).await;

    // Use a short timeout for testing (200ms)
    let timeout = Duration::from_millis(200);
    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        pool: app.pool.clone(),
        rig_service,
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
    let app = TestApp::new().await;
    let chat_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let rig_service = Arc::new(RigService::dummy());
    let (event_tx, _) = tokio::sync::broadcast::channel(100);

    // Use a 500ms timeout
    let timeout = Duration::from_millis(500);
    let handle = ChatActor::spawn(ChatActorArgs {
        chat_id,
        workspace_id,
        pool: app.pool.clone(),
        rig_service,
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
