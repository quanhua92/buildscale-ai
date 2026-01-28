//! Tests for touch tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool};

#[tokio::test]
async fn test_touch_new_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Touch New Test").await;
    
    let response = execute_tool(&app, &workspace_id, &token, "touch", serde_json::json!({
        "path": "/brand-new.txt"
    })).await;
    
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert!(!body["result"]["file_id"].as_str().unwrap().is_empty());
    
    // Verify file exists and is empty document
    let read_res = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({"path": "/brand-new.txt"})).await;
    assert_eq!(read_res.status(), 200);
    let read_body: serde_json::Value = read_res.json().await.unwrap();
    assert_eq!(read_body["result"]["content"], "");
}

#[tokio::test]
async fn test_touch_existing_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Touch Existing Test").await;
    
    // 1. Create file
    execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": "hello"
    })).await;
    
    // 2. Touch it
    let response = execute_tool(&app, &workspace_id, &token, "touch", serde_json::json!({
        "path": "/test.txt"
    })).await;
    
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
}
