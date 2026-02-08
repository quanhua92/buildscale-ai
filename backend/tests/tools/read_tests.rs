//! Tests for read tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_read_existing_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Test").await;

    let expected_content = "test content";
    write_file(&app, &workspace_id, &token, "/test.md", serde_json::json!(expected_content)).await;

    let actual_content = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/test.md"
    })).await;

    assert_eq!(actual_content.status(), 200);
    let body: serde_json::Value = actual_content.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["content"].as_str().unwrap(), expected_content);
}

#[tokio::test]
async fn test_read_nonexistent_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Nonexistent Test").await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/nonexistent.md"
    })).await;

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_read_deleted_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Deleted Test").await;

    write_file(&app, &workspace_id, &token, "/test.md", serde_json::json!("test content")).await;
    crate::tools::common::delete_file(&app, &workspace_id, &token, "/test.md").await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/test.md"
    })).await;

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_read_multiline_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Multiline Test").await;

    let content = "Title\n\nBody\n\nEnd";
    write_file(&app, &workspace_id, &token, "/doc.md", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/doc.md"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["content"], content);
}

#[tokio::test]
async fn test_read_with_default_limit() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Default Limit Test").await;

    // Create a file with 1000 lines
    let content: String = (0..1000).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/large.md", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/large.md"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should return only first 500 lines (default limit)
    let returned_content = body["result"]["content"].as_str().unwrap();
    let line_count = returned_content.lines().count();
    assert_eq!(line_count, 500);

    // Metadata should indicate truncation
    assert_eq!(body["result"]["total_lines"], 1000);
    assert_eq!(body["result"]["truncated"], true);
    assert_eq!(body["result"]["offset"], 0);
    assert_eq!(body["result"]["limit"], 500);
}

#[tokio::test]
async fn test_read_with_offset_and_limit() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Offset Limit Test").await;

    // Create a file with 100 lines
    let content: String = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md",
        "offset": 50,
        "limit": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should return lines 50-59 (10 lines total)
    let returned_content = body["result"]["content"].as_str().unwrap();
    let line_count = returned_content.lines().count();
    assert_eq!(line_count, 10);

    // Verify content starts with line 50
    assert!(returned_content.starts_with("Line 50"));

    // Metadata
    assert_eq!(body["result"]["total_lines"], 100);
    assert_eq!(body["result"]["truncated"], true); // More lines exist after line 59
    assert_eq!(body["result"]["offset"], 50);
    assert_eq!(body["result"]["limit"], 10);
}

#[tokio::test]
async fn test_read_with_offset_exceeds_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Offset Exceeds Test").await;

    let content = "Line 0\nLine 1\nLine 2";
    write_file(&app, &workspace_id, &token, "/small.md", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/small.md",
        "offset": 100,
        "limit": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should return empty content
    assert_eq!(body["result"]["content"].as_str().unwrap(), "");
    assert_eq!(body["result"]["total_lines"], 3);
    assert_eq!(body["result"]["truncated"], false);
}

#[tokio::test]
async fn test_read_with_zero_limit() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Zero Limit Test").await;

    let content = "Line 0\nLine 1\nLine 2";
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md",
        "limit": 0
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should return empty content
    assert_eq!(body["result"]["content"].as_str().unwrap(), "");
    assert_eq!(body["result"]["total_lines"], 3);
    assert_eq!(body["result"]["truncated"], true); // More lines exist
}

#[tokio::test]
async fn test_read_json_content_no_truncation() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read JSON Test").await;

    let json_content = serde_json::json!({
        "elements": ["a", "b", "c"],
        "nested": { "key": "value" }
    });
    write_file(&app, &workspace_id, &token, "/data.json", json_content.clone()).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/data.json",
        "limit": 1
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // JSON content should be returned as-is (no line truncation)
    assert_eq!(body["result"]["content"], json_content);
    // total_lines, truncated should be null for non-string content
    assert!(body["result"]["total_lines"].is_null());
    assert!(body["result"]["truncated"].is_null());
}

#[tokio::test]
async fn test_read_hash_unchanged_by_truncation() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Hash Test").await;

    let content = "Line 0\nLine 1\nLine 2\nLine 3\nLine 4";
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    // Read full file
    let full_response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md"
    })).await;
    let full_body: serde_json::Value = full_response.json().await.unwrap();
    let full_hash = full_body["result"]["hash"].as_str().unwrap();

    // Read truncated version
    let trunc_response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md",
        "limit": 2
    })).await;
    let trunc_body: serde_json::Value = trunc_response.json().await.unwrap();
    let trunc_hash = trunc_body["result"]["hash"].as_str().unwrap();

    // Hashes should be identical (computed from full content)
    assert_eq!(full_hash, trunc_hash);
}

#[tokio::test]
async fn test_read_with_negative_offset() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Negative Offset Test").await;

    // Create a file with 100 lines
    let content: String = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md",
        "offset": -10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should return last 10 lines (lines 90-99)
    let returned_content = body["result"]["content"].as_str().unwrap();
    assert!(returned_content.starts_with("Line 90"));
    assert_eq!(body["result"]["total_lines"], 100);
    assert_eq!(body["result"]["truncated"], false);
    assert_eq!(body["result"]["offset"], 90); // Actual position, not -10
}

#[tokio::test]
async fn test_read_negative_offset_exceeds_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Neg Offset Exceeds Test").await;

    let content = "Line 0\nLine 1\nLine 2";
    write_file(&app, &workspace_id, &token, "/small.md", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/small.md",
        "offset": -100
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should return all lines (offset -100 on 3-line file = start at 0)
    assert_eq!(body["result"]["content"].as_str().unwrap(), content);
    assert_eq!(body["result"]["total_lines"], 3);
    assert_eq!(body["result"]["truncated"], false);
    assert_eq!(body["result"]["offset"], 0);
}

#[tokio::test]
async fn test_read_negative_offset_with_limit() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Neg Offset Limit Test").await;

    // Create a file with 100 lines
    let content: String = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md",
        "offset": -50,
        "limit": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should read last 50 lines, but return only first 10 (lines 50-59)
    let returned_content = body["result"]["content"].as_str().unwrap();
    assert!(returned_content.starts_with("Line 50"));
    let line_count = returned_content.lines().count();
    assert_eq!(line_count, 10);
    assert_eq!(body["result"]["offset"], 50);
}

#[tokio::test]
async fn test_read_backward_compatibility_positive_offset() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Backward Compat Test").await;

    let content: String = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md",
        "offset": 10,
        "limit": 5
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Positive offset should still work: lines 10-14
    let returned_content = body["result"]["content"].as_str().unwrap();
    assert!(returned_content.starts_with("Line 10"));
    assert_eq!(body["result"]["offset"], 10);
}

// ============================================================================
// SCROLL MODE TESTS
// ============================================================================

#[tokio::test]
async fn test_read_scroll_mode_initial() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Scroll Mode Test").await;

    // Create a file with 100 lines
    let content: String = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    // Start scroll mode at line 50
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md",
        "cursor": 50,
        "offset": 0,
        "limit": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should read lines 50-59
    let returned_content = body["result"]["content"].as_str().unwrap();
    assert!(returned_content.starts_with("Line 50"));
    let line_count = returned_content.lines().count();
    assert_eq!(line_count, 10);

    // Cursor should be at end of read (line 60)
    assert_eq!(body["result"]["cursor"], 60);
    assert_eq!(body["result"]["offset"], 50);
}

#[tokio::test]
async fn test_read_scroll_mode_scroll_down() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Scroll Down Test").await;

    // Create a file with 100 lines
    let content: String = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    // Scroll down from cursor 50 with offset +10
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md",
        "cursor": 50,
        "offset": 10,
        "limit": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should read lines 60-69 (cursor 50 + offset 10)
    let returned_content = body["result"]["content"].as_str().unwrap();
    assert!(returned_content.starts_with("Line 60"));
    assert_eq!(body["result"]["offset"], 60);
    assert_eq!(body["result"]["cursor"], 70);
}

#[tokio::test]
async fn test_read_scroll_mode_scroll_up() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Scroll Up Test").await;

    // Create a file with 100 lines
    let content: String = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    // Scroll up from cursor 50 with offset -20
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md",
        "cursor": 50,
        "offset": -20,
        "limit": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should read lines 30-39 (cursor 50 - offset 20)
    let returned_content = body["result"]["content"].as_str().unwrap();
    assert!(returned_content.starts_with("Line 30"));
    assert_eq!(body["result"]["offset"], 30);
    assert_eq!(body["result"]["cursor"], 40);
}

#[tokio::test]
async fn test_read_scroll_mode_clamps_to_beginning() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Scroll Clamp Test").await;

    let content = "Line 0\nLine 1\nLine 2";
    write_file(&app, &workspace_id, &token, "/small.md", serde_json::json!(content)).await;

    // Try to scroll up from cursor 1 with offset -100 (should clamp to 0)
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/small.md",
        "cursor": 1,
        "offset": -100,
        "limit": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Should read from line 0 (clamped)
    assert_eq!(body["result"]["offset"], 0);
    assert_eq!(body["result"]["cursor"], 3); // 3 lines in file
}

#[tokio::test]
async fn test_read_cursor_field_always_returned() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Cursor Field Test").await;

    let content = "Line 0\nLine 1\nLine 2";
    write_file(&app, &workspace_id, &token, "/file.md", serde_json::json!(content)).await;

    // Read without cursor (absolute mode)
    let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/file.md"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Cursor field should always be returned (at end of read)
    assert_eq!(body["result"]["cursor"], 3);
}
