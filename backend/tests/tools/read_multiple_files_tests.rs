//! Tests for read_multiple_files tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_read_multiple_files_success() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Multiple Files Test").await;

    write_file(&app, &workspace_id, &token, "/file1.txt", serde_json::json!("content 1")).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!("content 2")).await;
    write_file(&app, &workspace_id, &token, "/file3.txt", serde_json::json!("content 3")).await;

    let response = execute_tool(&app, &workspace_id, &token, "read_multiple_files", serde_json::json!({
        "paths": ["/file1.txt", "/file2.txt", "/file3.txt"]
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let files = body["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 3);

    // All files should succeed
    for file in files {
        assert!(file["success"].as_bool().unwrap());
        assert!(file["content"].as_str().is_some() || file["content"].as_object().is_some());
        assert!(file["hash"].as_str().is_some());
        assert!(file["error"].is_null() || file["error"].as_str().is_none());
    }
}

#[tokio::test]
async fn test_read_multiple_files_partial_failure() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Multiple Partial Failure Test").await;

    write_file(&app, &workspace_id, &token, "/existing.txt", serde_json::json!("content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "read_multiple_files", serde_json::json!({
        "paths": ["/existing.txt", "/nonexistent.txt", "/another-missing.txt"]
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let files = body["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 3);

    // First file should succeed
    assert!(files[0]["success"].as_bool().unwrap());
    assert_eq!(files[0]["path"], "/existing.txt");

    // Other files should fail
    assert!(!files[1]["success"].as_bool().unwrap());
    assert_eq!(files[1]["path"], "/nonexistent.txt");
    assert!(files[1]["error"].as_str().is_some());

    assert!(!files[2]["success"].as_bool().unwrap());
    assert_eq!(files[2]["path"], "/another-missing.txt");
    assert!(files[2]["error"].as_str().is_some());
}

#[tokio::test]
async fn test_read_multiple_files_empty_paths() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Multiple Empty Test").await;

    let response = execute_tool(&app, &workspace_id, &token, "read_multiple_files", serde_json::json!({
        "paths": []
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_read_multiple_files_too_many_paths() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Multiple Too Many Test").await;

    // Create 51 paths (over the limit of 50)
    let paths: Vec<String> = (0..51).map(|i| format!("/file{}.txt", i)).collect();

    let response = execute_tool(&app, &workspace_id, &token, "read_multiple_files", serde_json::json!({
        "paths": paths
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["paths"].as_str().unwrap().contains("more than 50"));
}

#[tokio::test]
async fn test_read_multiple_files_with_limit() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Multiple Limit Test").await;

    // Create a file with 10 lines
    let content: String = (0..10).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "read_multiple_files", serde_json::json!({
        "paths": ["/file.txt"],
        "limit": 5
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let files = body["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);

    // Should only return first 5 lines
    let file_content = files[0]["content"].as_str().unwrap();
    let line_count = file_content.lines().count();
    assert_eq!(line_count, 5);
    assert_eq!(files[0]["total_lines"], 10);
    assert_eq!(files[0]["truncated"], true);
}

#[tokio::test]
async fn test_read_multiple_files_cannot_read_folder() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Read Multiple Folder Test").await;

    // Create a folder using mkdir
    let mkdir_response = execute_tool(&app, &workspace_id, &token, "mkdir", serde_json::json!({
        "path": "/folder"
    })).await;
    assert_eq!(mkdir_response.status(), 200);

    let response = execute_tool(&app, &workspace_id, &token, "read_multiple_files", serde_json::json!({
        "paths": ["/folder"]
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let files = body["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert!(!files[0]["success"].as_bool().unwrap());
    assert!(files[0]["error"].as_str().unwrap().contains("Cannot read a folder"));
}
