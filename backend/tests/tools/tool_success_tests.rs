//! Tests for tool success detection
//!
//! These tests verify that tool success/failure is correctly determined
//! and not falsely marked as failed when output contains "error:" text.

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_grep_success_with_error_in_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Test").await;

    // Create a file that mentions "Error:" in its content
    let content = r#"
// This file contains error handling code
function handleError() {
    console.error("Something went wrong");
    throw new Error("Test error");
}

// Another function
function validate() {
    if (!isValid) {
        return Error("Validation failed");
    }
}
"#;
    write_file(&app, &workspace_id, &token, "/code.js", serde_json::json!(content)).await;

    // Search for "Error" - should succeed even though content contains "Error:"
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "Error",
        "path_pattern": "*.js"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();

    // Tool should be marked as successful
    assert!(body["success"].as_bool().unwrap(), "Tool should succeed even when output contains 'Error:' text");

    // Should return matches
    let matches = body["result"]["matches"].as_array().unwrap();
    assert!(matches.len() > 0, "Should find matches for 'Error' pattern");
}

#[tokio::test]
async fn test_grep_success_with_error_word() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Test").await;

    // Create a README that mentions "Error" as a word
    let content = r#"
# Error Handling Guide

## Common Errors
- Error: File not found
- Error: Permission denied
- Error: Connection timeout

## Troubleshooting
When you see an Error message, check the logs.
"#;
    write_file(&app, &workspace_id, &token, "/README.md", serde_json::json!(content)).await;

    // Search for "Error" - should succeed
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "Error",
        "path_pattern": "*.md"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();

    // Tool should be marked as successful
    assert!(body["success"].as_bool().unwrap(), "Tool should succeed when finding 'Error' word in documentation");

    // Should return matches with context
    let matches = body["result"]["matches"].as_array().unwrap();
    assert!(matches.len() > 0);
}

#[tokio::test]
async fn test_read_tool_success_with_error_in_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Test").await;

    // Create a file that contains error logs
    let content = r#"
2025-01-01 10:00:00 ERROR: Database connection failed
2025-01-01 10:00:01 ERROR: Retry attempt 1
2025-01-01 10:00:02 ERROR: Retry attempt 2
2025-01-01 10:00:03 SUCCESS: Connection established
"#;
    write_file(&app, &workspace_id, &token, "/logs.txt", serde_json::json!(content)).await;

    // Read the file - should succeed even though it contains "ERROR:" lines
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/logs.txt"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();

    // Tool should be marked as successful
    assert!(body["success"].as_bool().unwrap(), "Read tool should succeed even when file contains 'ERROR:' text");

    // Content should include the ERROR lines
    let result_content = body["result"]["content"].as_str().unwrap();
    assert!(result_content.contains("ERROR: Database connection failed"));
}

#[tokio::test]
async fn test_grep_actual_tool_failure() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Test").await;

    // Try to search with a path pattern that escapes workspace (parent directory)
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "test",
        "path_pattern": "../etc/passwd"  // Invalid: parent directory reference
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();

    // Tool should be marked as failed
    assert_eq!(body["success"].as_bool().unwrap(), false, "Tool should fail for invalid path_pattern");

    // Should have an error message
    assert!(!body["error"].is_null(), "Should return error message for invalid path_pattern");

    let error_msg = body["error"].as_str().unwrap();
    assert!(error_msg.contains("parent directory"), "Error message should mention parent directory");
}

#[tokio::test]
async fn test_grep_no_matches_success() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Test").await;

    let content = "line 1\nline 2\nline 3";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // Search for something that doesn't exist
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "NOT_FOUND",
        "path_pattern": "*.txt"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();

    // Tool should be marked as successful (no matches is still a successful search)
    assert!(body["success"].as_bool().unwrap(), "Grep with no matches should still be successful");

    // Should return empty matches array
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 0, "Should return empty array for no matches");
}

#[tokio::test]
async fn test_grep_with_context_success() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Test").await;

    let content = "line 1\nline 2\nMATCH_HERE\nline 4\nline 5";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // Search with context - using string numbers (flexible parsing test too)
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "MATCH",
        "before_context": "2",
        "after_context": "2"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();

    // Tool should be marked as successful
    assert!(body["success"].as_bool().unwrap(), "Grep with context should succeed");

    // Should return match with context
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);

    let match_obj = &matches[0];
    assert!(!match_obj["before_context"].is_null(), "Should have before_context");
    assert!(!match_obj["after_context"].is_null(), "Should have after_context");
}
