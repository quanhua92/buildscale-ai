//! Tests for file_info tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_file_info_existing_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Test").await;

    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!("Hello, World!")).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/test.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["path"], "/test.txt");
    assert!(body["result"]["hash"].as_str().is_some());
    assert!(body["result"]["created_at"].as_str().is_some());
    assert!(body["result"]["updated_at"].as_str().is_some());
}

#[tokio::test]
async fn test_file_info_not_found() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Not Found Test").await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/nonexistent.txt"
    })).await;

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_file_info_line_count() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Line Count Test").await;

    let content = "line 1\nline 2\nline 3\nline 4\nline 5";
    write_file(&app, &workspace_id, &token, "/multiline.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/multiline.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["line_count"], 5);
}

#[tokio::test]
async fn test_file_info_size_accuracy() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Size Test").await;

    let content = "Hello, World!"; // 13 bytes
    write_file(&app, &workspace_id, &token, "/size_test.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/size_test.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["size"], content.len());
}

#[tokio::test]
async fn test_file_info_empty_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Empty Test").await;

    write_file(&app, &workspace_id, &token, "/empty.txt", serde_json::json!("")).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/empty.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["size"], 0);
    assert_eq!(body["result"]["line_count"], 0);
}

#[tokio::test]
async fn test_file_info_hash_consistency() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Hash Test").await;

    let content = "Consistent content for hash testing";
    write_file(&app, &workspace_id, &token, "/hash_test.txt", serde_json::json!(content)).await;

    // Call file_info twice and verify hash is the same
    let response1 = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/hash_test.txt"
    })).await;

    let response2 = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/hash_test.txt"
    })).await;

    assert_eq!(response1.status(), 200);
    assert_eq!(response2.status(), 200);

    let body1: serde_json::Value = response1.json().await.unwrap();
    let body2: serde_json::Value = response2.json().await.unwrap();

    let hash1 = body1["result"]["hash"].as_str().unwrap();
    let hash2 = body2["result"]["hash"].as_str().unwrap();

    assert_eq!(hash1, hash2, "Hash should be consistent across calls");
    assert_eq!(hash1.len(), 64, "SHA-256 hash should be 64 hex characters");
}

#[tokio::test]
async fn test_file_info_binary_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Binary Test").await;

    // Create binary content with null bytes and special characters
    let binary_content = "\x00\x01\x02\x03\u{FF}\u{FE}\u{FD}";
    write_file(&app, &workspace_id, &token, "/binary.bin", serde_json::json!(binary_content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/binary.bin"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["size"], binary_content.len());
    assert!(body["result"]["hash"].as_str().is_some());
}

#[tokio::test]
async fn test_file_info_large_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Large File Test").await;

    // Create a larger file (10KB)
    let large_content = "A".repeat(10_000);
    write_file(&app, &workspace_id, &token, "/large.txt", serde_json::json!(large_content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/large.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["size"], 10_000);
}

#[tokio::test]
async fn test_file_info_file_type_detection() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Type Test").await;

    write_file(&app, &workspace_id, &token, "/document.txt", serde_json::json!("text content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/document.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["file_type"], "document");
}

#[tokio::test]
async fn test_file_info_special_characters() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Special Chars Test").await;

    let special_content = "Line with tabs\tand newlines\nand special chars: ★ ♥ ♦";
    write_file(&app, &workspace_id, &token, "/special.txt", serde_json::json!(special_content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/special.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert!(body["result"]["hash"].as_str().is_some());
    // Verify special characters are handled correctly in hash
    let line_count = body["result"]["line_count"].as_u64().unwrap();
    assert!(line_count >= 1);
}

#[tokio::test]
async fn test_file_info_timestamps_present() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Timestamps Test").await;

    write_file(&app, &workspace_id, &token, "/timestamps.txt", serde_json::json!("test")).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/timestamps.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Verify all timestamp fields are present
    assert!(body["result"]["created_at"].as_str().is_some());
    assert!(body["result"]["updated_at"].as_str().is_some());

    // Verify timestamps are in ISO 8601 format
    let created_at = body["result"]["created_at"].as_str().unwrap();
    assert!(created_at.contains('T'), "created_at should be in ISO 8601 format");
    assert!(created_at.ends_with('Z'), "created_at should end with Z for UTC");
}

#[tokio::test]
async fn test_file_info_unicode_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Unicode Test").await;

    let unicode_content = "Hello 世界 🌍 Привет مرحبا";
    write_file(&app, &workspace_id, &token, "/unicode.txt", serde_json::json!(unicode_content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/unicode.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert!(body["result"]["hash"].as_str().is_some());
    // Size should be in bytes, not characters
    let size = body["result"]["size"].as_u64().unwrap();
    assert!(size > 0);
}

#[tokio::test]
async fn test_file_info_single_line() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "File Info Single Line Test").await;

    let single_line = "This is a single line without newline";
    write_file(&app, &workspace_id, &token, "/single.txt", serde_json::json!(single_line)).await;

    let response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
        "path": "/single.txt"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["line_count"], 1);
}
