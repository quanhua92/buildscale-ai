use buildscale::{
    models::agent_session::{AgentType, PauseSessionRequest, SessionStatus},
    services::agent_sessions::{
        create_session, get_session, list_workspace_sessions, list_user_sessions,
        update_session_status, update_session_task, pause_session, resume_session,
        cancel_session, delete_session,
    },
};
use crate::common::database::TestDb;
use uuid::Uuid;

/// Test successful session creation through service layer
#[tokio::test]
async fn test_service_create_session_success() {
    let test_db = TestDb::new("test_service_create_session_success").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create session through service layer
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    assert_eq!(session.workspace_id, workspace_id);
    assert_eq!(session.chat_id, chat_id);
    assert_eq!(session.user_id, user_id);
    assert_eq!(session.agent_type, AgentType::Assistant);
    assert_eq!(session.status, SessionStatus::Idle);
}

/// Test session creation validation - empty model
#[tokio::test]
async fn test_service_create_session_empty_model() {
    let test_db = TestDb::new("test_service_create_session_empty_model").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Try to create session with empty model
    let result = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "".to_string(),
        "chat".to_string(),
    )
    .await;

    assert!(result.is_err(), "Should fail validation for empty model");
}

/// Test session creation validation - invalid mode
#[tokio::test]
async fn test_service_create_session_invalid_mode() {
    let test_db = TestDb::new("test_service_create_session_invalid_mode").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Try to create session with invalid mode
    let result = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "invalid_mode".to_string(),
    )
    .await;

    assert!(result.is_err(), "Should fail validation for invalid mode");
}

/// Test getting session with authorization
#[tokio::test]
async fn test_service_get_session_with_auth() {
    let test_db = TestDb::new("test_service_get_session_with_auth").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data with two users
    let owner_id = Uuid::now_v7();
    let other_user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, owner_id, workspace_id).await;
    setup_user(&mut conn, other_user_id).await;
    setup_chat_file(&mut conn, workspace_id, chat_id).await;

    // Create session owned by owner_id
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        owner_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    // Owner can retrieve their own session
    let result = get_session(&mut conn, session.id, owner_id)
        .await;

    assert!(result.is_ok(), "Owner should be able to retrieve their session");

    // Other user cannot retrieve someone else's session
    let result = get_session(&mut conn, session.id, other_user_id)
        .await;

    assert!(result.is_err(), "Other user should not be able to retrieve the session");
}

/// Test listing workspace sessions
#[tokio::test]
async fn test_service_list_workspace_sessions() {
    let test_db = TestDb::new("test_service_list_workspace_sessions").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, user_id, workspace_id).await;

    // Create multiple sessions
    let chat1 = Uuid::now_v7();
    let chat2 = Uuid::now_v7();
    let chat3 = Uuid::now_v7();

    for chat_id in &[chat1, chat2, chat3] {
        setup_chat_file(&mut conn, workspace_id, *chat_id).await;
        create_session(
            &mut conn,
            workspace_id,
            *chat_id,
            user_id,
            AgentType::Assistant,
            "gpt-4o".to_string(),
            "chat".to_string(),
        )
        .await
        .expect("Session creation should succeed");
    }

    // List workspace sessions
    let response = list_workspace_sessions(&mut conn, workspace_id, user_id)
        .await
        .expect("Listing sessions should succeed");

    assert_eq!(response.sessions.len(), 3);
    assert_eq!(response.total, 3);
}

/// Test listing user sessions
#[tokio::test]
async fn test_service_list_user_sessions() {
    let test_db = TestDb::new("test_service_list_user_sessions").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data with two users in same workspace
    let user1_id = Uuid::now_v7();
    let user2_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, user1_id, workspace_id).await;
    setup_user(&mut conn, user2_id).await;

    // Create 2 sessions for user1
    let chat1 = Uuid::now_v7();
    let chat2 = Uuid::now_v7();
    setup_chat_file(&mut conn, workspace_id, chat1).await;
    setup_chat_file(&mut conn, workspace_id, chat2).await;

    create_session(
        &mut conn,
        workspace_id,
        chat1,
        user1_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .unwrap();

    create_session(
        &mut conn,
        workspace_id,
        chat2,
        user1_id,
        AgentType::Planner,
        "claude-3-5-sonnet".to_string(),
        "plan".to_string(),
    )
    .await
    .unwrap();

    // Create 1 session for user2
    let chat3 = Uuid::now_v7();
    setup_chat_file(&mut conn, workspace_id, chat3).await;

    create_session(
        &mut conn,
        workspace_id,
        chat3,
        user2_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .unwrap();

    // List user1's sessions
    let response = list_user_sessions(&mut conn, user1_id)
        .await
        .expect("Listing user sessions should succeed");

    assert_eq!(response.sessions.len(), 2, "User1 should have 2 sessions");
    assert_eq!(response.total, 2);
}

/// Test updating session status with transition validation
#[tokio::test]
async fn test_service_update_session_status_valid_transitions() {
    let test_db = TestDb::new("test_service_update_session_status_valid_transitions").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create session
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    assert_eq!(session.status, SessionStatus::Idle);

    // Valid transition: Idle -> Running
    let updated = update_session_status(
        &mut conn,
        session.id,
        SessionStatus::Running,
        user_id,
    )
    .await
    .expect("Valid transition should succeed");

    assert_eq!(updated.status, SessionStatus::Running);

    // Valid transition: Running -> Paused
    let paused = update_session_status(
        &mut conn,
        session.id,
        SessionStatus::Paused,
        user_id,
    )
    .await
    .expect("Valid transition should succeed");

    assert_eq!(paused.status, SessionStatus::Paused);

    // Valid transition: Paused -> Completed
    let completed = update_session_status(
        &mut conn,
        session.id,
        SessionStatus::Completed,
        user_id,
    )
    .await
    .expect("Valid transition should succeed");

    assert_eq!(completed.status, SessionStatus::Completed);
    assert!(completed.completed_at.is_some());
}

/// Test updating session status with invalid transitions
#[tokio::test]
async fn test_service_update_session_status_invalid_transitions() {
    let test_db = TestDb::new("test_service_update_session_status_invalid_transitions").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create session
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    // First transition to running
    let _running = update_session_status(
        &mut conn,
        session.id,
        SessionStatus::Running,
        user_id,
    )
    .await
    .expect("Transition to Running should succeed");

    // Then mark as completed
    let _completed = update_session_status(
        &mut conn,
        session.id,
        SessionStatus::Completed,
        user_id,
    )
    .await
    .expect("Transition to Completed should succeed");

    // Invalid transition: Completed -> Running
    let result = update_session_status(
        &mut conn,
        session.id,
        SessionStatus::Running,
        user_id,
    )
    .await;

    assert!(result.is_err(), "Should not allow transition from Completed to Running");
}

/// Test updating session task
#[tokio::test]
async fn test_service_update_session_task() {
    let test_db = TestDb::new("test_service_update_session_task").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create session
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Planner,
        "claude-3-5-sonnet".to_string(),
        "plan".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    // Update task
    let task = "Planning project architecture";
    let updated = update_session_task(
        &mut conn,
        session.id,
        Some(task.to_string()),
        user_id,
    )
    .await
    .expect("Task update should succeed");

    assert_eq!(updated.current_task, Some(task.to_string()));
}

/// Test pausing a session
#[tokio::test]
async fn test_service_pause_session() {
    let test_db = TestDb::new("test_service_pause_session").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create running session
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Builder,
        "gpt-4o".to_string(),
        "build".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    // Update to running
    update_session_status(&mut conn, session.id, SessionStatus::Running, user_id)
        .await
        .unwrap();

    // Pause session
    let request = PauseSessionRequest {
        reason: Some("User requested pause".to_string()),
    };

    let response = pause_session(&mut conn, session.id, request, user_id)
        .await
        .expect("Pausing should succeed");

    assert_eq!(response.session.status, SessionStatus::Paused);
    assert_eq!(response.message, "Session paused successfully");
}

/// Test pausing an already paused session
#[tokio::test]
async fn test_service_pause_already_paused() {
    let test_db = TestDb::new("test_service_pause_already_paused").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create paused session
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    update_session_status(&mut conn, session.id, SessionStatus::Paused, user_id)
        .await
        .unwrap();

    // Try to pause again
    let request = PauseSessionRequest {
        reason: None,
    };

    let result = pause_session(&mut conn, session.id, request, user_id).await;

    assert!(result.is_err(), "Should not be able to pause an already paused session");
}

/// Test resuming a paused session
#[tokio::test]
async fn test_service_resume_session() {
    let test_db = TestDb::new("test_service_resume_session").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create paused session
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    update_session_status(&mut conn, session.id, SessionStatus::Paused, user_id)
        .await
        .unwrap();

    // Resume session
    let response = resume_session(&mut conn, session.id, Some("New task".to_string()), user_id)
        .await
        .expect("Resume should succeed");

    assert_eq!(response.session.status, SessionStatus::Idle);
    assert_eq!(response.session.current_task, Some("New task".to_string()));
    assert_eq!(response.message, "Session resumed successfully");
}

/// Test resuming a non-paused session
#[tokio::test]
async fn test_service_resume_not_paused() {
    let test_db = TestDb::new("test_service_resume_not_paused").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create running session
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    // Try to resume running session
    let result = resume_session(&mut conn, session.id, None, user_id).await;

    assert!(result.is_err(), "Should not be able to resume a running session");
}

/// Test cancelling a session
#[tokio::test]
async fn test_service_cancel_session() {
    let test_db = TestDb::new("test_service_cancel_session").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create running session
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    update_session_status(&mut conn, session.id, SessionStatus::Running, user_id)
        .await
        .unwrap();

    // Cancel session
    let response = cancel_session(&mut conn, session.id, user_id)
        .await
        .expect("Cancellation should succeed");

    assert_eq!(response.session.status, SessionStatus::Completed);
    assert_eq!(response.message, "Session cancelled successfully");
}

/// Test deleting a session
#[tokio::test]
async fn test_service_delete_session() {
    let test_db = TestDb::new("test_service_delete_session").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id).await;

    // Create session
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        user_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    // Delete session
    delete_session(&mut conn, session.id, user_id)
        .await
        .expect("Deletion should succeed");

    // Verify session is gone
    let result = get_session(&mut conn, session.id, user_id).await;

    assert!(result.is_err(), "Session should be deleted");
}

/// Test deleting someone else's session
#[tokio::test]
async fn test_service_delete_unauthorized_session() {
    let test_db = TestDb::new("test_service_delete_unauthorized_session").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data with two users
    let owner_id = Uuid::now_v7();
    let other_user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, owner_id, workspace_id).await;
    setup_user(&mut conn, other_user_id).await;
    setup_chat_file(&mut conn, workspace_id, chat_id).await;

    // Create session owned by owner_id
    let session = create_session(
        &mut conn,
        workspace_id,
        chat_id,
        owner_id,
        AgentType::Assistant,
        "gpt-4o".to_string(),
        "chat".to_string(),
    )
    .await
    .expect("Session creation should succeed");

    // Try to delete with other user
    let result = delete_session(&mut conn, session.id, other_user_id).await;

    assert!(result.is_err(), "Other user should not be able to delete the session");
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

async fn setup_test_data(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    user_id: Uuid,
    workspace_id: Uuid,
    chat_id: Uuid,
) {
    setup_user_and_workspace(conn, user_id, workspace_id).await;
    setup_chat_file(conn, workspace_id, chat_id).await;
}

async fn setup_user_and_workspace(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    user_id: Uuid,
    workspace_id: Uuid,
) {
    setup_user(conn, user_id).await;

    sqlx::query(
        r#"
        INSERT INTO workspaces (id, name, owner_id, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
    )
    .bind(workspace_id)
    .bind("Test Workspace")
    .bind(user_id)
    .execute(conn.as_mut())
    .await
    .unwrap();
}

async fn setup_user(conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
    )
    .bind(user_id)
    .bind(format!("test_service_session_{}@example.com", user_id))
    .bind("hash")
    .execute(conn.as_mut())
    .await
    .unwrap();
}

async fn setup_chat_file(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    workspace_id: Uuid,
    chat_id: Uuid,
) {
    let file_name = format!("test_chat_{}.md", chat_id);
    let file_path = format!("/{}", file_name);
    sqlx::query(
        r#"
        INSERT INTO files (id, workspace_id, name, slug, path, file_type, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
        "#,
    )
    .bind(chat_id)
    .bind(workspace_id)
    .bind(&file_name)
    .bind(&file_name)
    .bind(&file_path)
    .bind("chat")
    .execute(conn.as_mut())
    .await
    .unwrap();
}
