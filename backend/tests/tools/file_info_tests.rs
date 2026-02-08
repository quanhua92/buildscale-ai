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

// Skip folder test - file_info is primarily for files, not folders
// Use ls tool to inspect folders instead
