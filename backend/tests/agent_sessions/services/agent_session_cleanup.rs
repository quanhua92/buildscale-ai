use buildscale::{
    models::agent_session::{AgentType, SessionStatus},
    queries::agent_sessions::{get_session_by_id},
    services::agent_sessions::{create_session as service_create_session, cleanup_stale_sessions},
};
use crate::common::database::TestDb;
use uuid::Uuid;
use chrono::{Utc, Duration};

/// Test cleanup of stale sessions
#[tokio::test]
async fn test_service_cleanup_stale_sessions() {
    let test_db = TestDb::new("test_service_cleanup_stale_sessions").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data with proper test prefix
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, user_id, workspace_id, &test_db.test_prefix()).await;

    // Create 3 sessions with different heartbeat times
    let mut session_ids = Vec::new();

    for i in 0..3 {
        let chat_id = Uuid::now_v7();
        setup_chat_file(&mut conn, workspace_id, chat_id).await;

        let session = service_create_session(
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

        session_ids.push(session.id);

        // Manually set heartbeat based on index
        let heartbeat_time = match i {
            0 => Utc::now() - Duration::seconds(200), // Stale (200 seconds ago)
            1 => Utc::now() - Duration::seconds(150), // Stale (150 seconds ago)
            _ => Utc::now() - Duration::seconds(30),  // Fresh (30 seconds ago)
        };

        sqlx::query(
            r#"
            UPDATE agent_sessions
            SET last_heartbeat = $1
            WHERE id = $2
            "#,
        )
        .bind(heartbeat_time)
        .bind(session.id)
        .execute(conn.as_mut())
        .await
        .unwrap();
    }

    // Run cleanup
    let cleaned_count = cleanup_stale_sessions(&mut conn)
        .await
        .expect("Cleanup should succeed");

    // Should clean up 2 stale sessions (the ones with heartbeat > 120 seconds ago)
    assert_eq!(cleaned_count, 2, "Should clean up 2 stale sessions");

    // Verify the fresh session still exists
    let fresh_session = get_session_by_id(&mut conn, session_ids[2])
        .await
        .expect("Query should succeed");

    assert!(fresh_session.is_some(), "Fresh session should still exist");

    // Verify stale sessions are gone
    for session_id in &session_ids[0..2] {
        let result = get_session_by_id(&mut conn, *session_id)
            .await
            .expect("Query should succeed");

        assert!(result.is_none(), "Stale session should be deleted");
    }
}

/// Test that completed and error sessions are not cleaned up
#[tokio::test]
async fn test_service_cleanup_does_not_remove_terminal_states() {
    let test_db = TestDb::new("test_service_cleanup_does_not_remove_terminal_states").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, user_id, workspace_id, &test_db.test_prefix()).await;

    // Create sessions with terminal states and old heartbeats
    for status in &[SessionStatus::Completed, SessionStatus::Error] {
        let chat_id = Uuid::now_v7();
        setup_chat_file(&mut conn, workspace_id, chat_id).await;

        let session = service_create_session(
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

        // Update to terminal state
        sqlx::query(
            r#"
            UPDATE agent_sessions
            SET status = $1, completed_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(status)
        .bind(session.id)
        .execute(conn.as_mut())
        .await
        .unwrap();

        // Set old heartbeat
        let old_heartbeat = Utc::now() - Duration::seconds(200);
        sqlx::query(
            r#"
            UPDATE agent_sessions
            SET last_heartbeat = $1
            WHERE id = $2
            "#,
        )
        .bind(old_heartbeat)
        .bind(session.id)
        .execute(conn.as_mut())
        .await
        .unwrap();
    }

    // Run cleanup
    let cleaned_count = cleanup_stale_sessions(&mut conn)
        .await
        .expect("Cleanup should succeed");

    // Terminal state sessions should NOT be cleaned up
    assert_eq!(cleaned_count, 0, "Should not clean up terminal state sessions");
}

/// Test cleanup with no stale sessions
#[tokio::test]
async fn test_service_cleanup_with_fresh_sessions() {
    let test_db = TestDb::new("test_service_cleanup_with_fresh_sessions").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();

    setup_user_and_workspace(&mut conn, user_id, workspace_id, &test_db.test_prefix()).await;

    // Create fresh session
    let chat_id = Uuid::now_v7();
    setup_chat_file(&mut conn, workspace_id, chat_id).await;

    service_create_session(
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

    // Run cleanup
    let cleaned_count = cleanup_stale_sessions(&mut conn)
        .await
        .expect("Cleanup should succeed");

    assert_eq!(cleaned_count, 0, "Should not clean up fresh sessions");
}

/// Test cleanup handles empty database gracefully
#[tokio::test]
async fn test_service_cleanup_empty_database() {
    let test_db = TestDb::new("test_service_cleanup_empty_database").await;
    let mut conn = test_db.get_connection().await;

    // Run cleanup on empty database
    let cleaned_count = cleanup_stale_sessions(&mut conn)
        .await
        .expect("Cleanup should succeed even with no sessions");

    assert_eq!(cleaned_count, 0, "Should clean up 0 sessions from empty database");
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

async fn setup_user_and_workspace(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    user_id: Uuid,
    workspace_id: Uuid,
    test_prefix: &str,
) {
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
