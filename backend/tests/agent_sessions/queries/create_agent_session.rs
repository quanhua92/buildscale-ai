use buildscale::{
    models::agent_session::{AgentType, NewAgentSession, SessionStatus},
    queries::agent_sessions::{create_session, get_session_by_id},
};
use crate::common::database::TestDb;

/// Test successful agent session creation
#[tokio::test]
async fn test_create_agent_session_success() {
    let test_db = TestDb::new("test_create_agent_session_success").await;
    let mut conn = test_db.get_connection().await;

    // First, create a user, workspace, and chat file (required foreign keys)
    let user_id = uuid::Uuid::now_v7();
    let workspace_id = uuid::Uuid::now_v7();
    let chat_id = uuid::Uuid::now_v7();

    // Create user (direct SQL for test setup)
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
    )
    .bind(user_id)
    .bind(format!("test_create_session_{}@example.com", user_id))
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

    // Create agent session
    let new_session = NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: AgentType::Assistant,
        model: "gpt-4o".to_string(),
        mode: "chat".to_string(),
    };

    let created_session = create_session(&mut conn, new_session)
        .await
        .expect("Session creation should succeed");

    // Verify session was created correctly
    assert_eq!(created_session.workspace_id, workspace_id);
    assert_eq!(created_session.chat_id, chat_id);
    assert_eq!(created_session.user_id, user_id);
    assert_eq!(created_session.agent_type, AgentType::Assistant);
    assert_eq!(created_session.status, SessionStatus::Idle);
    assert_eq!(created_session.model, "gpt-4o");
    assert_eq!(created_session.mode, "chat");
    assert!(created_session.current_task.is_none());
    assert!(created_session.completed_at.is_none());

    // Verify session can be retrieved
    let retrieved = get_session_by_id(&mut conn, created_session.id)
        .await
        .expect("Session should be retrievable")
        .expect("Session should exist");

    assert_eq!(retrieved.id, created_session.id);
}

/// Test duplicate chat_id constraint violation
#[tokio::test]
async fn test_create_agent_session_duplicate_chat_id() {
    let test_db = TestDb::new("test_create_agent_session_duplicate_chat_id").await;
    let mut conn = test_db.get_connection().await;

    // Setup user, workspace, and chat
    let user_id = uuid::Uuid::now_v7();
    let workspace_id = uuid::Uuid::now_v7();
    let chat_id = uuid::Uuid::now_v7();

    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
    )
    .bind(user_id)
    .bind(format!("test_duplicate_chat_{}@example.com", user_id))
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
    .bind("Test Workspace")
    .bind(user_id)
    .execute(conn.as_mut())
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO files (id, workspace_id, name, slug, path, file_type, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
        "#,
    )
    .bind(chat_id)
    .bind(workspace_id)
    .bind("test_chat.md")
    .bind("test_chat.md")
    .bind("/test_chat.md")
    .bind("chat")
    .execute(conn.as_mut())
    .await
    .unwrap();

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

    // Attempt to create second session with same chat_id
    let result = create_session(&mut conn, new_session).await;

    assert!(result.is_err(), "Should fail with Conflict error for duplicate chat_id");
}

/// Test foreign key constraint violations
#[tokio::test]
async fn test_create_agent_session_foreign_key_violations() {
    let test_db = TestDb::new("test_create_agent_session_foreign_key_violations").await;
    let mut conn = test_db.get_connection().await;

    let user_id = uuid::Uuid::now_v7();
    let workspace_id = uuid::Uuid::now_v7();
    let chat_id = uuid::Uuid::now_v7();

    // Attempt to create session with non-existent entities
    let new_session = NewAgentSession {
        workspace_id,
        chat_id,
        user_id,
        agent_type: AgentType::Assistant,
        model: "gpt-4o".to_string(),
        mode: "chat".to_string(),
    };

    let result = create_session(&mut conn, new_session).await;

    assert!(result.is_err(), "Should fail with foreign key constraint violation");
}
