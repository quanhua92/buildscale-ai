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

