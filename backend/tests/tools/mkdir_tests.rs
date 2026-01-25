//! Tests for mkdir tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool};

#[tokio::test]
async fn test_mkdir_basic() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Mkdir Basic Test").await;
    
    // 1. Create a directory
    let response = execute_tool(&app, &workspace_id, &token, "mkdir", serde_json::json!({
        "path": "/my-folder"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["path"], "/my-folder");
    assert!(!body["result"]["file_id"].is_null());

    // 2. Verify it exists via ls
    let ls_res = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/"
    })).await;
    let ls_body: serde_json::Value = ls_res.json().await.unwrap();
    let entries = ls_body["result"]["entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["name"] == "my-folder" && e["file_type"] == "folder"));
}

#[tokio::test]
async fn test_mkdir_recursive() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Mkdir Recursive Test").await;
    
    // 1. Create nested directories
    let response = execute_tool(&app, &workspace_id, &token, "mkdir", serde_json::json!({
        "path": "/a/b/c"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["path"], "/a/b/c");

    // 2. Verify structure via ls
    let ls_res = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/a/b"
    })).await;
    let ls_body: serde_json::Value = ls_res.json().await.unwrap();
    let entries = ls_body["result"]["entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["name"] == "c" && e["file_type"] == "folder"));
}

#[tokio::test]
async fn test_mkdir_conflict_with_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Mkdir Conflict Test").await;
    
    // 1. Create a file
    execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": "hello"
    })).await;

    // 2. Try to mkdir at same path
    let response = execute_tool(&app, &workspace_id, &token, "mkdir", serde_json::json!({
        "path": "/test.txt"
    })).await;

    assert_eq!(response.status(), 409); // Conflict
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "CONFLICT");
    assert!(body["error"].as_str().unwrap().contains("is not a folder"));
}

#[tokio::test]
async fn test_mkdir_idempotent() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Mkdir Idempotent Test").await;
    
    // 1. Create a directory
    let res1 = execute_tool(&app, &workspace_id, &token, "mkdir", serde_json::json!({
        "path": "/folder"
    })).await;
    let body1: serde_json::Value = res1.json().await.unwrap();
    let id1 = body1["result"]["file_id"].as_str().unwrap();

    // 2. Mkdir again
    let res2 = execute_tool(&app, &workspace_id, &token, "mkdir", serde_json::json!({
        "path": "/folder"
    })).await;
    let body2: serde_json::Value = res2.json().await.unwrap();
    let id2 = body2["result"]["file_id"].as_str().unwrap();

    assert_eq!(id1, id2);
}
