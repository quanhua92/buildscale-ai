//! Tests for memory tools (memory_set, memory_get, memory_search, memory_delete)
//!
//! Tests cover:
//! - User-scoped memory creation and retrieval
//! - Global-scoped memory creation and retrieval
//! - Memory search with filtering
//! - Memory deletion
//! - User isolation (users cannot access other users' memories)
//! - Memory updates

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::execute_tool;

/// Helper to set a memory
async fn set_memory(
    app: &TestApp,
    workspace_id: &str,
    token: &str,
    scope: &str,
    category: &str,
    key: &str,
    title: &str,
    content: &str,
    tags: Option<Vec<&str>>,
) -> serde_json::Value {
    let mut args = serde_json::json!({
        "scope": scope,
        "category": category,
        "key": key,
        "title": title,
        "content": content
    });

    if let Some(t) = tags {
        args["tags"] = serde_json::json!(t);
    }

    let response = execute_tool(app, workspace_id, token, "memory_set", args).await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// Helper to get a memory
async fn get_memory(
    app: &TestApp,
    workspace_id: &str,
    token: &str,
    scope: &str,
    category: &str,
    key: &str,
) -> serde_json::Value {
    let response = execute_tool(
        app,
        workspace_id,
        token,
        "memory_get",
        serde_json::json!({
            "scope": scope,
            "category": category,
            "key": key
        }),
    )
    .await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// Helper to search memories
async fn search_memories(
    app: &TestApp,
    workspace_id: &str,
    token: &str,
    pattern: &str,
    scope: Option<&str>,
    category: Option<&str>,
    tags: Option<Vec<&str>>,
) -> serde_json::Value {
    let mut args = serde_json::json!({
        "pattern": pattern
    });

    if let Some(s) = scope {
        args["scope"] = serde_json::json!(s);
    }
    if let Some(c) = category {
        args["category"] = serde_json::json!(c);
    }
    if let Some(t) = tags {
        args["tags"] = serde_json::json!(t);
    }

    let response = execute_tool(app, workspace_id, token, "memory_search", args).await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

/// Helper to delete a memory
async fn delete_memory(
    app: &TestApp,
    workspace_id: &str,
    token: &str,
    scope: &str,
    category: &str,
    key: &str,
) -> serde_json::Value {
    let response = execute_tool(
        app,
        workspace_id,
        token,
        "memory_delete",
        serde_json::json!({
            "scope": scope,
            "category": category,
            "key": key
        }),
    )
    .await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

// ============================================================================
// User-Scoped Memory Tests
// ============================================================================

#[tokio::test]
async fn test_memory_set_user_scope() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory User Test").await;

    let result = set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "preferences",
        "coding-style",
        "Coding Style",
        "User prefers TypeScript with strict mode enabled.",
        Some(vec!["coding", "typescript"]),
    )
    .await;

    assert!(result["success"].as_bool().unwrap());
    assert!(!result["result"]["file_id"].as_str().unwrap().is_empty());
    assert!(result["result"]["path"].as_str().unwrap().contains("/users/"));
    assert!(result["result"]["path"].as_str().unwrap().contains("/memories/"));
}

#[tokio::test]
async fn test_memory_get_user_scope() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Get User Test").await;

    // Set a memory first
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "project-notes",
        "Project Notes",
        "Important project information here.",
        None,
    )
    .await;

    // Get the memory
    let result = get_memory(&app, &workspace_id, &token, "user", "work", "project-notes").await;

    assert!(result["success"].as_bool().unwrap());
    assert!(result["result"]["content"].as_str().unwrap().contains("Important project information"));
    assert_eq!(result["result"]["metadata"]["title"].as_str().unwrap(), "Project Notes");
    assert_eq!(result["result"]["metadata"]["category"].as_str().unwrap(), "work");
    assert_eq!(result["result"]["key"].as_str().unwrap(), "project-notes");
}

// ============================================================================
// Global-Scoped Memory Tests
// ============================================================================

#[tokio::test]
async fn test_memory_set_global_scope() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Global Test").await;

    let result = set_memory(
        &app,
        &workspace_id,
        &token,
        "global",
        "team",
        "guidelines",
        "Team Guidelines",
        "Team coding guidelines for all members.",
        Some(vec!["team", "guidelines"]),
    )
    .await;

    assert!(result["success"].as_bool().unwrap());
    assert!(result["result"]["path"].as_str().unwrap().starts_with("/memories/"));
    assert!(!result["result"]["path"].as_str().unwrap().contains("/users/"));
}

#[tokio::test]
async fn test_memory_get_global_scope() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Get Global Test").await;

    // Set a global memory
    set_memory(
        &app,
        &workspace_id,
        &token,
        "global",
        "config",
        "api-keys",
        "API Keys",
        "List of API keys for the project.",
        None,
    )
    .await;

    // Get the memory
    let result = get_memory(&app, &workspace_id, &token, "global", "config", "api-keys").await;

    assert!(result["success"].as_bool().unwrap());
    assert!(result["result"]["content"].as_str().unwrap().contains("API keys"));
    assert_eq!(result["result"]["metadata"]["scope"].as_str().unwrap(), "global");
}

// ============================================================================
// Search Tests
// ============================================================================

#[tokio::test]
async fn test_memory_search_pattern() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Search Test").await;

    // Set some memories
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "notes-1",
        "Notes 1",
        "TypeScript is a typed superset of JavaScript.",
        None,
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "notes-2",
        "Notes 2",
        "Python is great for data science.",
        None,
    )
    .await;

    // Search for TypeScript
    let result = search_memories(&app, &workspace_id, &token, "TypeScript", None, None, None).await;

    assert!(result["success"].as_bool().unwrap());
    assert!(result["result"]["total"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn test_memory_search_filter_scope() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Scope Filter Test").await;

    // Set both user and global memories with same keyword
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "test",
        "user-note",
        "User Note",
        "Test keyword content.",
        None,
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "global",
        "test",
        "global-note",
        "Global Note",
        "Test keyword content.",
        None,
    )
    .await;

    // Search only user scope
    let result = search_memories(&app, &workspace_id, &token, "keyword", Some("user"), None, None).await;

    assert!(result["success"].as_bool().unwrap());
    // All results should be user-scoped
    for m in result["result"]["matches"].as_array().unwrap() {
        assert_eq!(m["scope"].as_str().unwrap(), "user");
    }
}

#[tokio::test]
async fn test_memory_search_filter_category() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Category Filter Test").await;

    // Set memories in different categories
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "personal",
        "note-1",
        "Personal",
        "UniqueContentXYZ personal note.",
        None,
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "note-2",
        "Work",
        "UniqueContentXYZ work note.",
        None,
    )
    .await;

    // Search only in personal category
    let result = search_memories(
        &app,
        &workspace_id,
        &token,
        "UniqueContentXYZ",
        None,
        Some("personal"),
        None,
    )
    .await;

    assert!(result["success"].as_bool().unwrap());
    // All results should be in personal category
    for m in result["result"]["matches"].as_array().unwrap() {
        assert_eq!(m["category"].as_str().unwrap(), "personal");
    }
}

#[tokio::test]
async fn test_memory_search_filter_tags() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Tags Filter Test").await;

    // Set memories with different tags
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "tagged-note-1",
        "Tagged Note 1",
        "SearchTagTest content with frontend tag.",
        Some(vec!["frontend", "react"]),
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "tagged-note-2",
        "Tagged Note 2",
        "SearchTagTest content with backend tag.",
        Some(vec!["backend", "rust"]),
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "tagged-note-3",
        "Tagged Note 3",
        "SearchTagTest content with both tags.",
        Some(vec!["frontend", "backend"]),
    )
    .await;

    // Search for memories with "frontend" tag
    let result = search_memories(
        &app,
        &workspace_id,
        &token,
        "SearchTagTest",
        None,
        None,
        Some(vec!["frontend"]),
    )
    .await;

    assert!(result["success"].as_bool().unwrap());
    // Results should have frontend tag
    for m in result["result"]["matches"].as_array().unwrap() {
        let tags = m["tags"].as_array().unwrap();
        assert!(tags.iter().any(|t| t.as_str().unwrap() == "frontend"));
    }
}

// ============================================================================
// User Isolation Tests
// ============================================================================

#[tokio::test]
async fn test_memory_user_isolation() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;

    // Create two users
    let token1 = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token1, "Isolation Test").await;

    let token2 = register_and_login(&app).await;

    // User 1 creates a user-scoped memory
    set_memory(
        &app,
        &workspace_id,
        &token1,
        "user",
        "private",
        "secret",
        "Secret",
        "User 1's secret content.",
        None,
    )
    .await;

    // User 2 tries to access User 1's memory - should fail
    let response = execute_tool(
        &app,
        &workspace_id,
        &token2,
        "memory_get",
        serde_json::json!({
            "scope": "user",
            "category": "private",
            "key": "secret"
        }),
    )
    .await;

    // Should return 403 Forbidden (user2 is trying to access user1's memory)
    // or 404 Not Found (depending on how the path is resolved)
    // Both are acceptable for isolation
    assert!(response.status() == 403 || response.status() == 404,
        "Expected 403 or 404, got {}", response.status());
}

// ============================================================================
// Update Tests
// ============================================================================

#[tokio::test]
async fn test_memory_update() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Update Test").await;

    // Create initial memory
    let result1 = set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "updatable",
        "Initial Title",
        "Initial content.",
        None,
    )
    .await;
    let file_id1 = result1["result"]["file_id"].as_str().unwrap();

    // Update memory with same key
    let result2 = set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "updatable",
        "Updated Title",
        "Updated content with new information.",
        Some(vec!["updated"]),
    )
    .await;
    let file_id2 = result2["result"]["file_id"].as_str().unwrap();

    // File ID should be the same (update, not create)
    assert_eq!(file_id1, file_id2);

    // Get and verify updated content
    let result = get_memory(&app, &workspace_id, &token, "user", "work", "updatable").await;

    assert!(result["result"]["content"].as_str().unwrap().contains("Updated content"));
    assert_eq!(result["result"]["metadata"]["title"].as_str().unwrap(), "Updated Title");
}

// ============================================================================
// Limit Tests
// ============================================================================

#[tokio::test]
async fn test_memory_search_limit() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Limit Test").await;

    // Create multiple memories with same keyword
    for i in 0..10 {
        set_memory(
            &app,
            &workspace_id,
            &token,
            "user",
            "test",
            &format!("limit-test-{}", i),
            &format!("Limit Test {}", i),
            &format!("LimitTestKeyword content number {}.", i),
            None,
        )
        .await;
    }

    // Search with limit
    let response = execute_tool(
        &app,
        &workspace_id,
        &token,
        "memory_search",
        serde_json::json!({
            "pattern": "LimitTestKeyword",
            "limit": 3
        }),
    )
    .await;

    assert_eq!(response.status(), 200);
    let result: serde_json::Value = response.json().await.unwrap();

    assert!(result["success"].as_bool().unwrap());
    assert_eq!(result["result"]["matches"].as_array().unwrap().len(), 3);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_memory_get_not_found() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Not Found Test").await;

    // Try to get a memory that doesn't exist
    let response = execute_tool(
        &app,
        &workspace_id,
        &token,
        "memory_get",
        serde_json::json!({
            "scope": "user",
            "category": "nonexistent",
            "key": "key"
        }),
    )
    .await;

    // Should return 404 Not Found
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_memory_set_missing_fields() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Validation Test").await;

    // Missing required fields
    let response = execute_tool(
        &app,
        &workspace_id,
        &token,
        "memory_set",
        serde_json::json!({
            "scope": "user",
            "category": "test"
            // Missing: key, title, content
        }),
    )
    .await;

    // Should return error
    assert_ne!(response.status(), 200);
}

// ============================================================================
// Delete Tests
// ============================================================================

#[tokio::test]
async fn test_memory_delete_user_scope() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Delete User Test").await;

    // Create a memory first
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "test",
        "to-delete",
        "To Delete",
        "This memory will be deleted.",
        None,
    )
    .await;

    // Delete the memory
    let result = delete_memory(&app, &workspace_id, &token, "user", "test", "to-delete").await;

    assert!(result["success"].as_bool().unwrap());
    assert!(result["result"]["file_id"].as_str().unwrap().len() > 0);
    assert_eq!(result["result"]["scope"].as_str().unwrap(), "user");
    assert_eq!(result["result"]["category"].as_str().unwrap(), "test");
    assert_eq!(result["result"]["key"].as_str().unwrap(), "to-delete");

    // Verify memory is no longer accessible
    let response = execute_tool(
        &app,
        &workspace_id,
        &token,
        "memory_get",
        serde_json::json!({
            "scope": "user",
            "category": "test",
            "key": "to-delete"
        }),
    )
    .await;

    // Should return 404 Not Found
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_memory_delete_global_scope() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Delete Global Test").await;

    // Create a global memory
    set_memory(
        &app,
        &workspace_id,
        &token,
        "global",
        "team",
        "guidelines",
        "Team Guidelines",
        "Team coding guidelines.",
        None,
    )
    .await;

    // Delete the memory
    let result = delete_memory(&app, &workspace_id, &token, "global", "team", "guidelines").await;

    assert!(result["success"].as_bool().unwrap());
    assert_eq!(result["result"]["scope"].as_str().unwrap(), "global");

    // Verify memory is no longer accessible
    let response = execute_tool(
        &app,
        &workspace_id,
        &token,
        "memory_get",
        serde_json::json!({
            "scope": "global",
            "category": "team",
            "key": "guidelines"
        }),
    )
    .await;

    // Should return 404 Not Found
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_memory_delete_not_found() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory Delete Not Found Test").await;

    // Try to delete a memory that doesn't exist
    let response = execute_tool(
        &app,
        &workspace_id,
        &token,
        "memory_delete",
        serde_json::json!({
            "scope": "user",
            "category": "nonexistent",
            "key": "key"
        }),
    )
    .await;

    // Should return 404 Not Found
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_memory_delete_user_isolation() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;

    // Create two users
    let token1 = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token1, "Delete Isolation Test").await;

    let token2 = register_and_login(&app).await;

    // User 1 creates a user-scoped memory
    set_memory(
        &app,
        &workspace_id,
        &token1,
        "user",
        "private",
        "secret",
        "Secret",
        "User 1's secret content.",
        None,
    )
    .await;

    // User 2 tries to delete User 1's memory - should fail
    let response = execute_tool(
        &app,
        &workspace_id,
        &token2,
        "memory_delete",
        serde_json::json!({
            "scope": "user",
            "category": "private",
            "key": "secret"
        }),
    )
    .await;

    // Should return 403 or 404 (user2 can't access user1's memory)
    assert!(response.status() == 403 || response.status() == 404,
        "Expected 403 or 404, got {}", response.status());
}
