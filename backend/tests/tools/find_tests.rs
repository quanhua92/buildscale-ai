//! Tests for find tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_find_by_name_pattern() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find Name Pattern Test").await;

    write_file(&app, &workspace_id, &token, "/test1.txt", serde_json::json!("content 1")).await;
    write_file(&app, &workspace_id, &token, "/test2.txt", serde_json::json!("content 2")).await;
    write_file(&app, &workspace_id, &token, "/other.md", serde_json::json!("markdown")).await;

    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.txt"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    // Should find all .txt files
    assert!(matches.len() >= 2);
    let names: Vec<&str> = matches.iter().map(|m| m["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"test1.txt"));
    assert!(names.contains(&"test2.txt"));
    // other.md should not be included
    assert!(!names.contains(&"other.md"));
}

#[tokio::test]
async fn test_find_by_file_type() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find File Type Test").await;

    write_file(&app, &workspace_id, &token, "/file1.txt", serde_json::json!("text")).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!("text")).await;

    let mkdir_response = execute_tool(&app, &workspace_id, &token, "mkdir", serde_json::json!({
        "path": "/folder"
    })).await;
    assert_eq!(mkdir_response.status(), 200);

    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "file_type": "folder"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["file_type"], "folder");
}

#[tokio::test]
async fn test_find_empty() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find Empty Test").await;

    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!("content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "nonexistent*"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 0);
}

#[tokio::test]
async fn test_find_recursive_default() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find Recursive Test").await;

    write_file(&app, &workspace_id, &token, "/folder/nested/file.txt", serde_json::json!("content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.txt"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["path"], "/folder/nested/file.txt");
}
