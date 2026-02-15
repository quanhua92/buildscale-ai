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

// ============================================================================
// Memory List Tests
// ============================================================================

/// Helper to list memories (categories, tags, or memories)
async fn list_memories(
    app: &TestApp,
    workspace_id: &str,
    token: &str,
    list_type: &str,
    scope: Option<&str>,
    category: Option<&str>,
    tags: Option<Vec<&str>>,
    limit: Option<usize>,
) -> serde_json::Value {
    let mut args = serde_json::json!({
        "list_type": list_type
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
    if let Some(l) = limit {
        args["limit"] = serde_json::json!(l);
    }

    let response = execute_tool(app, workspace_id, token, "memory_list", args).await;
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

#[tokio::test]
async fn test_memory_list_categories() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory List Categories Test").await;

    // Create memories in different categories
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "project-a",
        "Project A",
        "Work on project A.",
        None,
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "project-b",
        "Project B",
        "Work on project B.",
        None,
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "personal",
        "hobby",
        "Hobby",
        "Personal hobby notes.",
        None,
    )
    .await;

    // List categories
    let result = list_memories(&app, &workspace_id, &token, "categories", None, None, None, None).await;

    assert!(result["success"].as_bool().unwrap());
    let categories = result["result"]["categories"].as_array().unwrap();
    assert!(categories.len() >= 2);

    // Verify categories exist
    let category_names: Vec<&str> = categories
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(category_names.contains(&"work"));
    assert!(category_names.contains(&"personal"));
}

#[tokio::test]
async fn test_memory_list_categories_with_scope_filter() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory List Categories Scope Test").await;

    // Create user and global memories
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "private",
        "user-note",
        "User Note",
        "Private user note.",
        None,
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "global",
        "team",
        "team-note",
        "Team Note",
        "Shared team note.",
        None,
    )
    .await;

    // List only user categories
    let result = list_memories(&app, &workspace_id, &token, "categories", Some("user"), None, None, None).await;

    assert!(result["success"].as_bool().unwrap());
    let categories = result["result"]["categories"].as_array().unwrap();

    // Should only contain user-scoped categories
    for cat in categories {
        assert_eq!(cat["name"].as_str().unwrap(), "private");
    }
}

#[tokio::test]
async fn test_memory_list_tags() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory List Tags Test").await;

    // Create memories with different tags
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "tagged-a",
        "Tagged A",
        "Content with tags.",
        Some(vec!["frontend", "react"]),
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "tagged-b",
        "Tagged B",
        "Content with other tags.",
        Some(vec!["backend", "rust"]),
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "tagged-c",
        "Tagged C",
        "Content with shared tag.",
        Some(vec!["frontend", "typescript"]),
    )
    .await;

    // List tags
    let result = list_memories(&app, &workspace_id, &token, "tags", None, None, None, None).await;

    assert!(result["success"].as_bool().unwrap());
    let tags = result["result"]["tags"].as_array().unwrap();
    assert!(tags.len() >= 3);

    // Verify tags exist and have counts
    let tag_names: Vec<&str> = tags
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(tag_names.contains(&"frontend"));
    assert!(tag_names.contains(&"backend"));
    assert!(tag_names.contains(&"react"));
}

#[tokio::test]
async fn test_memory_list_memories() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory List Memories Test").await;

    // Create memories
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "list-test-1",
        "List Test 1",
        "First memory for list test.",
        Some(vec!["test"]),
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "list-test-2",
        "List Test 2",
        "Second memory for list test.",
        Some(vec!["test"]),
    )
    .await;

    // List memories
    let result = list_memories(&app, &workspace_id, &token, "memories", None, None, None, None).await;

    assert!(result["success"].as_bool().unwrap());
    let memories = result["result"]["memories"].as_array().unwrap();
    assert!(memories.len() >= 2);

    // Verify memories have correct structure
    for mem in memories {
        assert!(mem["path"].as_str().unwrap().contains("/memories/"));
        assert!(!mem["key"].as_str().unwrap().is_empty());
        assert!(!mem["title"].as_str().unwrap().is_empty());
        assert!(mem["tags"].as_array().unwrap().len() > 0 || mem["tags"].as_array().unwrap().is_empty());
    }
}

#[tokio::test]
async fn test_memory_list_memories_with_filters() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory List Memories Filter Test").await;

    // Create memories in different categories
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "category-a",
        "mem-filter-1",
        "Filter Test 1",
        "Memory in category A.",
        Some(vec!["filter-test"]),
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "category-b",
        "mem-filter-2",
        "Filter Test 2",
        "Memory in category B.",
        Some(vec!["filter-test"]),
    )
    .await;

    // List memories filtered by category
    let result = list_memories(
        &app,
        &workspace_id,
        &token,
        "memories",
        None,
        Some("category-a"),
        None,
        None,
    )
    .await;

    assert!(result["success"].as_bool().unwrap());
    let memories = result["result"]["memories"].as_array().unwrap();

    // All results should be in category-a
    for mem in memories {
        assert_eq!(mem["category"].as_str().unwrap(), "category-a");
    }
}

#[tokio::test]
async fn test_memory_list_memories_with_tags_filter() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory List Tags Filter Test").await;

    // Create memories with different tag combinations
    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "tags-filter-1",
        "Tags Filter 1",
        "Memory with frontend and react.",
        Some(vec!["frontend", "react"]),
    )
    .await;

    set_memory(
        &app,
        &workspace_id,
        &token,
        "user",
        "work",
        "tags-filter-2",
        "Tags Filter 2",
        "Memory with frontend only.",
        Some(vec!["frontend"]),
    )
    .await;

    // List memories with tags filter (AND logic)
    let result = list_memories(
        &app,
        &workspace_id,
        &token,
        "memories",
        None,
        None,
        Some(vec!["frontend", "react"]),
        None,
    )
    .await;

    assert!(result["success"].as_bool().unwrap());
    let memories = result["result"]["memories"].as_array().unwrap();

    // All results should have both tags
    for mem in memories {
        let tags = mem["tags"].as_array().unwrap();
        let tag_names: Vec<&str> = tags.iter().map(|t| t.as_str().unwrap()).collect();
        assert!(tag_names.contains(&"frontend"));
        assert!(tag_names.contains(&"react"));
    }
}

#[tokio::test]
async fn test_memory_list_limit() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Memory List Limit Test").await;

    // Create multiple memories
    for i in 0..10 {
        set_memory(
            &app,
            &workspace_id,
            &token,
            "user",
            "limit-test",
            &format!("limit-mem-{}", i),
            &format!("Limit Memory {}", i),
            &format!("Content {} for limit test.", i),
            None,
        )
        .await;
    }

    // List with limit
    let result = list_memories(&app, &workspace_id, &token, "memories", None, Some("limit-test"), None, Some(3)).await;

    assert!(result["success"].as_bool().unwrap());
    let memories = result["result"]["memories"].as_array().unwrap();
    assert_eq!(memories.len(), 3);
}

