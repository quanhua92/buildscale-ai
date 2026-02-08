//! Tests for cat tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_cat_multiple_files() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Multiple Files Test").await;

    write_file(&app, &workspace_id, &token, "/file1.txt", serde_json::json!("line 1\nline 2")).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!("line 3\nline 4")).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file1.txt", "/file2.txt"]
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let content = body["result"]["content"].as_str().unwrap();
    assert!(content.contains("line 1"));
    assert!(content.contains("line 3"));

    let files = body["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
}

#[tokio::test]
async fn test_cat_with_headers() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat With Headers Test").await;

    write_file(&app, &workspace_id, &token, "/file1.txt", serde_json::json!("content")).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!("content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file1.txt", "/file2.txt"],
        "show_headers": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let content = body["result"]["content"].as_str().unwrap();
    assert!(content.contains("==> /file1.txt <=="));
    assert!(content.contains("==> /file2.txt <=="));
}

#[tokio::test]
async fn test_cat_with_line_numbers() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Line Numbers Test").await;

    let content = "line 1\nline 2\nline 3";
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "number_lines": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let content = body["result"]["content"].as_str().unwrap();
    assert!(content.contains("1\tline 1"));
    assert!(content.contains("2\tline 2"));
    assert!(content.contains("3\tline 3"));
}

#[tokio::test]
async fn test_cat_empty_paths() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Empty Paths Test").await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": []
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_cat_too_many_paths() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Too Many Paths Test").await;

    // Create 21 paths (over the limit of 20)
    let paths: Vec<String> = (0..21).map(|i| format!("/file{}.txt", i)).collect();

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": paths
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_cat_partial_failure() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Partial Failure Test").await;

    write_file(&app, &workspace_id, &token, "/existing.txt", serde_json::json!("content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/existing.txt", "/nonexistent.txt"]
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let files = body["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);

    // First file should succeed
    assert_eq!(files[0]["path"], "/existing.txt");
    assert_eq!(files[0]["line_count"], 1);

    // Second file should fail
    assert_eq!(files[1]["path"], "/nonexistent.txt");
    let content = files[1]["content"].as_str().unwrap();
    assert!(content.contains("Error reading"));
}
