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

#[tokio::test]
async fn test_find_min_size() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find Min Size Test").await;

    write_file(&app, &workspace_id, &token, "/small.txt", serde_json::json!("hi")).await;
    write_file(&app, &workspace_id, &token, "/medium.txt", serde_json::json!("Hello World!")).await;
    write_file(&app, &workspace_id, &token, "/large.txt", serde_json::json!("This is a much larger file with more content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.txt",
        "min_size": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    // Should only find files with size >= 10 bytes
    assert!(matches.len() >= 1);
    for file_match in matches.iter() {
        let size = file_match["size"].as_u64().unwrap();
        assert!(size >= 10, "File {} has size {}, should be >= 10", file_match["path"], size);
    }
}

#[tokio::test]
async fn test_find_max_size() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find Max Size Test").await;

    write_file(&app, &workspace_id, &token, "/tiny.txt", serde_json::json!("a")).await;
    write_file(&app, &workspace_id, &token, "/small.txt", serde_json::json!("hello")).await;
    write_file(&app, &workspace_id, &token, "/big.txt", serde_json::json!("This is a very large file with lots and lots of content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.txt",
        "max_size": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    // Should only find files with size <= 10 bytes
    assert!(matches.len() >= 1);
    for file_match in matches.iter() {
        let size = file_match["size"].as_u64().unwrap();
        assert!(size <= 10, "File {} has size {}, should be <= 10", file_match["path"], size);
    }
}

#[tokio::test]
async fn test_find_size_range() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find Size Range Test").await;

    write_file(&app, &workspace_id, &token, "/tiny.txt", serde_json::json!("a")).await; // 1 byte
    write_file(&app, &workspace_id, &token, "/medium.txt", serde_json::json!("Hello World!")).await; // 12 bytes
    write_file(&app, &workspace_id, &token, "/large.txt", serde_json::json!("This is a very large file with lots and lots of content")).await; // 61 bytes

    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.txt",
        "min_size": 5,
        "max_size": 20
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    // Should only find files with 5 <= size <= 20 bytes
    assert!(matches.len() >= 1);
    for file_match in matches.iter() {
        let size = file_match["size"].as_u64().unwrap();
        assert!(size >= 5 && size <= 20, "File {} has size {}, should be between 5 and 20", file_match["path"], size);
    }
}

#[tokio::test]
async fn test_find_combined_filters() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find Combined Filters Test").await;

    write_file(&app, &workspace_id, &token, "/doc1.txt", serde_json::json!("small")).await;
    write_file(&app, &workspace_id, &token, "/doc2.txt", serde_json::json!("This is a larger text file")).await;
    write_file(&app, &workspace_id, &token, "/other.md", serde_json::json!("This is a larger markdown file")).await;

    let mkdir_response = execute_tool(&app, &workspace_id, &token, "mkdir", serde_json::json!({
        "path": "/folder"
    })).await;
    assert_eq!(mkdir_response.status(), 200);

    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.txt",
        "min_size": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    // Should only find .txt files with size >= 10
    assert!(matches.len() >= 1);
    for file_match in matches.iter() {
        let path = file_match["path"].as_str().unwrap();
        let size = file_match["size"].as_u64().unwrap();

        assert!(path.ends_with(".txt"), "File {} should end with .txt", path);
        assert!(size >= 10, "File {} has size {}, should be >= 10", path, size);
    }
}

#[tokio::test]
async fn test_find_non_recursive() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find Non-Recursive Test").await;

    write_file(&app, &workspace_id, &token, "/root.txt", serde_json::json!("root content")).await;
    write_file(&app, &workspace_id, &token, "/folder/nested.txt", serde_json::json!("nested content")).await;

    // Search non-recursively from root
    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.txt",
        "path": "/",
        "recursive": false
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    // Should only find root.txt, not nested.txt
    assert!(matches.len() >= 1);
    let paths: Vec<&str> = matches.iter().map(|m| m["path"].as_str().unwrap()).collect();
    assert!(paths.contains(&"/root.txt"));
    assert!(!paths.contains(&"/folder/nested.txt"), "Should not find nested files when recursive=false");
}

#[tokio::test]
async fn test_find_with_path_parameter() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find Path Parameter Test").await;

    write_file(&app, &workspace_id, &token, "/folder1/file1.txt", serde_json::json!("content 1")).await;
    write_file(&app, &workspace_id, &token, "/folder2/file2.txt", serde_json::json!("content 2")).await;

    let response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.txt",
        "path": "/folder1"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    // Should only find files in folder1
    assert!(matches.len() >= 1);
    for file_match in matches.iter() {
        let path = file_match["path"].as_str().unwrap();
        assert!(path.starts_with("/folder1"), "File {} should be in /folder1", path);
    }
}
