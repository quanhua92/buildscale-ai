//! Tests for flexible number/boolean parsing
//!
//! These tests verify that the custom deserializers correctly handle
//! AI-generated JSON where numbers are sent as strings.

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_grep_flexible_context_parsing_string() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Test").await;

    let content = "line 1\nline 2\nMATCH\nline 4\nline 5";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // AI sends context as string "5" instead of integer 5
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "MATCH",
        "before_context": "5",
        "after_context": "5"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
}

#[tokio::test]
async fn test_grep_flexible_context_parsing_integer() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Test").await;

    let content = "line 1\nline 2\nMATCH\nline 4\nline 5";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // Standard integer format (should still work)
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "MATCH",
        "before_context": 2,
        "after_context": 2
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
}

#[tokio::test]
async fn test_grep_flexible_context_shorthand_string() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Test").await;

    let content = "line 1\nline 2\nMATCH\nline 4\nline 5";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // AI sends context shorthand as string
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "MATCH",
        "context": "3"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
}

#[tokio::test]
async fn test_read_flexible_offset_string() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Test").await;

    let content = (1..=100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // AI sends offset as string "10"
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/test.txt",
        "offset": "10"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let result_content = body["result"]["content"].as_str().unwrap();
    assert!(result_content.starts_with("line 11"));
}

#[tokio::test]
async fn test_read_flexible_limit_string() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Test").await;

    let content = (1..=100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // AI sends limit as string "10"
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/test.txt",
        "limit": "10"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let lines = body["result"]["content"].as_str().unwrap().lines().count();
    assert_eq!(lines, 10);
}

#[tokio::test]
async fn test_read_flexible_negative_offset_string() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Test").await;

    let content = (1..=100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // AI sends negative offset as string "-10"
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/test.txt",
        "offset": "-10"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let result_content = body["result"]["content"].as_str().unwrap();
    assert!(result_content.contains("line 91"));
}

#[tokio::test]
async fn test_read_flexible_cursor_string() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Test").await;

    let content = (1..=100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // AI sends cursor as string "50"
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/test.txt",
        "cursor": "50",
        "offset": 0,
        "limit": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let result_content = body["result"]["content"].as_str().unwrap();
    assert!(result_content.starts_with("line 51"));
}

#[tokio::test]
async fn test_grep_flexible_mixed_string_int() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Test").await;

    let content = "line 1\nline 2\nMATCH\nline 4\nline 5";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // AI sends before_context as string "3" and after_context as integer 1
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "MATCH",
        "before_context": "3",
        "after_context": 1
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
}

#[tokio::test]
async fn test_read_flexible_all_params_string() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Test").await;

    let content = (1..=100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // AI sends all numeric params as strings
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/test.txt",
        "offset": "50",
        "limit": "10",
        "cursor": "55"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
}
