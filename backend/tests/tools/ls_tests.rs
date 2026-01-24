//! Tests for ls tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_ls_root_directory() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Test").await;
    
    write_file(&app, &workspace_id, &token, "/file1.md", serde_json::json!({"text": "content1"})).await;
    write_file(&app, &workspace_id, &token, "/file2.md", serde_json::json!({"text": "content2"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({})).await;
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["entries"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_ls_nested_directory() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Nested Test").await;
    
    write_file(&app, &workspace_id, &token, "/folder/nested.md", serde_json::json!({"text": "nested"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/folder"
    })).await;
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(body["result"]["entries"][0]["name"], "nested.md");
}

#[tokio::test]
async fn test_ls_recursive() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Recursive Test").await;
    
    write_file(&app, &workspace_id, &token, "/folder/subfolder/nested.md", serde_json::json!({"text": "nested"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/folder",
        "recursive": true
    })).await;
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["entries"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_ls_empty_directory() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Empty Test").await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({})).await;
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_ls_nonexistent_path() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Nonexistent Test").await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/nonexistent"
    })).await;
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_ls_file_as_directory() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS File Validation Test").await;
    
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "content"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/test.txt"
    })).await;
    
    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["path"].as_str().unwrap().contains("Path is not a directory"));
}
