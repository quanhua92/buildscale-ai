use buildscale::{
    models::agent_session::{AgentType, NewAgentSession, SessionStatus},
    queries::agent_sessions::{
        create_session, get_session_by_id, update_session_status, update_session_task,
        update_heartbeat, get_active_sessions_by_workspace, cleanup_stale_sessions,
        get_workspace_session_stats,
    },
};
use crate::common::database::TestDb;
use uuid::Uuid;
use chrono::{Utc, Duration};

/// Test updating session status
#[tokio::test]
async fn test_update_session_status_success() {
    let test_db = TestDb::new("test_update_session_status_success").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id, test_db.test_prefix()).await;

    // Create session
    let new_session = NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: AgentType::Assistant,
        model: "gpt-4o".to_string(),
        mode: "chat".to_string(),
    };

    let created = create_session(&mut conn, new_session)
        .await
        .expect("Session creation should succeed");

    assert_eq!(created.status, SessionStatus::Idle);

    // Update status to Running
    let updated = update_session_status(&mut conn, created.id, SessionStatus::Running)
        .await
        .expect("Status update should succeed");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.status, SessionStatus::Running);
    assert!(updated.completed_at.is_none());

    // Update status to Completed
    let completed = update_session_status(&mut conn, created.id, SessionStatus::Completed)
        .await
        .expect("Status update to Completed should succeed");

    assert_eq!(completed.status, SessionStatus::Completed);
    assert!(completed.completed_at.is_some(), "completed_at should be set");
}

/// Test updating session task
#[tokio::test]
async fn test_update_session_task_success() {
    let test_db = TestDb::new("test_update_session_task_success").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id, test_db.test_prefix()).await;

    // Create session
    let new_session = NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: AgentType::Planner,
        model: "claude-3-5-sonnet".to_string(),
        mode: "plan".to_string(),
    };

    let created = create_session(&mut conn, new_session)
        .await
        .expect("Session creation should succeed");

    assert!(created.current_task.is_none());

    // Update task
    let task_description = "Planning project architecture";
    let updated = update_session_task(&mut conn, created.id, Some(task_description.to_string()))
        .await
        .expect("Task update should succeed");

    assert_eq!(updated.current_task, Some(task_description.to_string()));

    // Clear task
    let cleared = update_session_task(&mut conn, created.id, None)
        .await
        .expect("Task clearing should succeed");

    assert!(cleared.current_task.is_none());
}

/// Test updating session heartbeat
#[tokio::test]
async fn test_update_heartbeat_success() {
    let test_db = TestDb::new("test_update_heartbeat_success").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_data(&mut conn, user_id, workspace_id, chat_id, test_db.test_prefix()).await;

    // Create session
    let new_session = NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: AgentType::Builder,
        model: "gpt-4o".to_string(),
        mode: "build".to_string(),
    };

    let created = create_session(&mut conn, new_session)
        .await
        .expect("Session creation should succeed");

    let original_heartbeat = created.last_heartbeat;

    // Wait a bit to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Update heartbeat
    update_heartbeat(&mut conn, created.id)
        .await
        .expect("Heartbeat update should succeed");

    // Verify heartbeat was updated
    let retrieved = get_session_by_id(&mut conn, created.id)
        .await
        .expect("Session retrieval should succeed")
        .expect("Session should exist");

    assert!(retrieved.last_heartbeat > original_heartbeat, "Heartbeat should be updated");
}

/// Test getting active sessions for a workspace
#[tokio::test]
async fn test_get_active_sessions_by_workspace() {
    let test_db = TestDb::new("test_get_active_sessions_by_workspace").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, user_id, workspace_id, test_db.test_prefix()).await;

    // Create multiple chat files and sessions
    let chat1 = Uuid::now_v7();
    let chat2 = Uuid::now_v7();
    let chat3 = Uuid::now_v7();

    setup_chat_file(&mut conn, workspace_id, chat1).await;
    setup_chat_file(&mut conn, workspace_id, chat2).await;
    setup_chat_file(&mut conn, workspace_id, chat3).await;

    // Create 3 sessions
    for chat_id in &[chat1, chat2, chat3] {
        let new_session = NewAgentSession {
            workspace_id,
            chat_id: *chat_id,
            user_id,
            agent_type: AgentType::Assistant,
            model: "gpt-4o".to_string(),
            mode: "chat".to_string(),
        };

        create_session(&mut conn, new_session)
            .await
            .expect("Session creation should succeed");
    }

    // Get active sessions
    let active_sessions = get_active_sessions_by_workspace(&mut conn, workspace_id)
        .await
        .expect("Getting active sessions should succeed");

    assert_eq!(active_sessions.len(), 3, "Should have 3 active sessions");

    // Mark one as completed
    let first_session = &active_sessions[0];
    update_session_status(&mut conn, first_session.id, SessionStatus::Completed)
        .await
        .expect("Status update should succeed");

    // Get active sessions again
    let active_sessions_after = get_active_sessions_by_workspace(&mut conn, workspace_id)
        .await
        .expect("Getting active sessions should succeed");

    assert_eq!(active_sessions_after.len(), 2, "Should have 2 active sessions after one completed");
}

/// Test cleanup of stale sessions
#[tokio::test]
async fn test_cleanup_stale_sessions() {
    let test_db = TestDb::new("test_cleanup_stale_sessions").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, user_id, workspace_id, test_db.test_prefix()).await;

    // Create a chat file and session
    let chat_id = Uuid::now_v7();
    setup_chat_file(&mut conn, workspace_id, chat_id).await;

    let new_session = NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: AgentType::Assistant,
        model: "gpt-4o".to_string(),
        mode: "chat".to_string(),
    };

    let created = create_session(&mut conn, new_session)
        .await
        .expect("Session creation should succeed");

    // Manually set heartbeat to old time (beyond stale threshold)
    let old_heartbeat = Utc::now() - Duration::seconds(200); // 200 seconds ago
    sqlx::query(
        r#"
        UPDATE agent_sessions
        SET last_heartbeat = $1
        WHERE id = $2
        "#,
    )
    .bind(old_heartbeat)
    .bind(created.id)
    .execute(conn.as_mut())
    .await
    .unwrap();

    // Run cleanup - should clean up at least our session
    let cleaned_count = cleanup_stale_sessions(&mut conn)
        .await
        .expect("Cleanup should succeed");

    assert!(cleaned_count >= 1, "Should clean up at least 1 stale session, got {}", cleaned_count);

    // Verify session was deleted
    let retrieved = get_session_by_id(&mut conn, created.id)
        .await
        .expect("Query should succeed");

    assert!(retrieved.is_none(), "Session should be deleted");
}

/// Test getting workspace session statistics
#[tokio::test]
async fn test_get_workspace_session_stats() {
    let test_db = TestDb::new("test_get_workspace_session_stats").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, user_id, workspace_id, test_db.test_prefix()).await;

    // Create multiple sessions with different statuses
    let sessions_to_create = vec![
        (SessionStatus::Idle, AgentType::Assistant),
        (SessionStatus::Idle, AgentType::Planner),
        (SessionStatus::Running, AgentType::Builder),
        (SessionStatus::Paused, AgentType::Assistant),
        (SessionStatus::Completed, AgentType::Planner),
        (SessionStatus::Error, AgentType::Builder),
    ];

    for (status, agent_type) in sessions_to_create {
        let chat_id = Uuid::now_v7();
        setup_chat_file(&mut conn, workspace_id, chat_id).await;

        let new_session = NewAgentSession {
            workspace_id,
            chat_id,
            user_id,
            agent_type,
            model: "gpt-4o".to_string(),
            mode: "chat".to_string(),
        };

        let created = create_session(&mut conn, new_session)
            .await
            .expect("Session creation should succeed");

        // Update status if not idle
        if status != SessionStatus::Idle {
            update_session_status(&mut conn, created.id, status)
                .await
                .expect("Status update should succeed");
        }
    }

    // Get statistics
    let stats = get_workspace_session_stats(&mut conn, workspace_id)
        .await
        .expect("Getting stats should succeed");

    assert_eq!(stats.total, 6, "Total should be 6");
    assert_eq!(stats.idle, 2, "Should have 2 idle sessions");
    assert_eq!(stats.running, 1, "Should have 1 running session");
    assert_eq!(stats.paused, 1, "Should have 1 paused session");
    assert_eq!(stats.completed, 1, "Should have 1 completed session");
    assert_eq!(stats.error, 1, "Should have 1 error session");
}

/// Test updating non-existent session
#[tokio::test]
async fn test_update_nonexistent_session() {
    let test_db = TestDb::new("test_update_nonexistent_session").await;
    let mut conn = test_db.get_connection().await;

    let fake_id = Uuid::now_v7();

    // Try to update non-existent session
    let result = update_session_status(&mut conn, fake_id, SessionStatus::Running).await;

    assert!(result.is_err(), "Should return error for non-existent session");
}

/// Helper function to set up complete test data
async fn setup_test_data(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    user_id: Uuid,
    workspace_id: Uuid,
    chat_id: Uuid,
    test_prefix: &str,
) {
    setup_user_and_workspace(conn, user_id, workspace_id, test_prefix).await;
    setup_chat_file(conn, workspace_id, chat_id).await;
}

/// Helper to set up user and workspace
async fn setup_user_and_workspace(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    user_id: Uuid,
    workspace_id: Uuid,
    test_prefix: &str,
) {
    // Create user
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
    )
    .bind(user_id)
    .bind(format!("{}@example.com", test_prefix))
    .bind("hash")
    .execute(conn.as_mut())
    .await
    .unwrap();

    // Create workspace
    sqlx::query(
        r#"
        INSERT INTO workspaces (id, name, owner_id, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
    )
    .bind(workspace_id)
    .bind(test_prefix)
    .bind(user_id)
    .execute(conn.as_mut())
    .await
    .unwrap();
}

/// Helper to set up chat file
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
