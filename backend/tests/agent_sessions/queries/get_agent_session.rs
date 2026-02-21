use buildscale::{
    models::agent_session::{AgentType, NewAgentSession},
    queries::agent_sessions::{create_session, get_session_by_id, get_session_by_chat},
};
use crate::common::database::TestDb;
use uuid::Uuid;

/// Test getting session by ID
#[tokio::test]
async fn test_get_session_by_id_found() {
    let test_db = TestDb::new("test_get_session_by_id_found").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_user_workspace_chat(&mut conn, user_id, workspace_id, chat_id).await;

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

    // Get session by ID
    let retrieved = get_session_by_id(&mut conn, created.id)
        .await
        .expect("Session retrieval should succeed")
        .expect("Session should exist");

    assert_eq!(retrieved.id, created.id);
    assert_eq!(retrieved.chat_id, chat_id);
    assert_eq!(retrieved.agent_type, AgentType::Planner);
    assert_eq!(retrieved.mode, "plan");
}

/// Test getting non-existent session by ID
#[tokio::test]
async fn test_get_session_by_id_not_found() {
    let test_db = TestDb::new("test_get_session_by_id_not_found").await;
    let mut conn = test_db.get_connection().await;

    let fake_id = Uuid::now_v7();
    let result = get_session_by_id(&mut conn, fake_id).await;

    assert!(result.is_ok(), "Query should not error");
    assert!(result.unwrap().is_none(), "Should return None for non-existent session");
}

/// Test getting session by chat ID
#[tokio::test]
async fn test_get_session_by_chat_found() {
    let test_db = TestDb::new("test_get_session_by_chat_found").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_user_workspace_chat(&mut conn, user_id, workspace_id, chat_id).await;

    // Create session
    let new_session = NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: AgentType::Builder,
        model: "gpt-4o".to_string(),
        mode: "build".to_string(),
    };

    create_session(&mut conn, new_session)
        .await
        .expect("Session creation should succeed");

    // Get session by chat ID
    let retrieved = get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Session retrieval should succeed");

    assert!(retrieved.is_some(), "Should find session by chat_id");
    let session = retrieved.unwrap();
    assert_eq!(session.chat_id, chat_id);
    assert_eq!(session.agent_type, AgentType::Builder);
    assert_eq!(session.mode, "build");
}

/// Test getting non-existent session by chat ID
#[tokio::test]
async fn test_get_session_by_chat_not_found() {
    let test_db = TestDb::new("test_get_session_by_chat_not_found").await;
    let mut conn = test_db.get_connection().await;

    let fake_chat_id = Uuid::now_v7();
    let result = get_session_by_chat(&mut conn, fake_chat_id).await;

    assert!(result.is_ok(), "Query should not error");
    assert!(result.unwrap().is_none(), "Should return None for non-existent chat");
}

/// Test that chat_id uniqueness constraint prevents multiple sessions per chat
#[tokio::test]
async fn test_chat_id_uniqueness() {
    let test_db = TestDb::new("test_chat_id_uniqueness").await;
    let mut conn = test_db.get_connection().await;

    // Setup test data
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let chat_id = Uuid::now_v7();

    setup_test_user_workspace_chat(&mut conn, user_id, workspace_id, chat_id).await;

    // Create first session
    let new_session = NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: AgentType::Assistant,
        model: "gpt-4o".to_string(),
        mode: "chat".to_string(),
    };

    create_session(&mut conn, new_session.clone())
        .await
        .expect("First session creation should succeed");

    // Attempt to create second session with same chat
    let result = create_session(&mut conn, new_session).await;
    assert!(result.is_err(), "Second session with same chat_id should fail");

    // Verify only one session exists for this chat
    let retrieved = get_session_by_chat(&mut conn, chat_id)
        .await
        .expect("Should retrieve the single session");

    assert!(retrieved.is_some(), "One session should exist");
}

/// Helper function to set up test user, workspace, and chat
async fn setup_test_user_workspace_chat(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    user_id: Uuid,
    workspace_id: Uuid,
    chat_id: Uuid,
) {
    // Create user
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
    )
    .bind(user_id)
    .bind(format!("test_get_session_{}@example.com", user_id))
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
    .bind("Test Workspace")
    .bind(user_id)
    .execute(conn.as_mut())
    .await
    .unwrap();

    // Create chat file
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
