//! Tests for write tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file, read_file};

#[tokio::test]
async fn test_write_new_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Test").await;

    let content = "new file content";
    let file_id = write_file(&app, &workspace_id, &token, "/new.txt", serde_json::json!(content)).await;

    assert!(!file_id.is_empty());

    let read_content = read_file(&app, &workspace_id, &token, "/new.txt").await;
    assert_eq!(read_content.as_str().unwrap(), content);
}

#[tokio::test]
async fn test_write_update_existing_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Update Test").await;

    let initial_content = "initial";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(initial_content)).await;

    let updated_content = "updated";
    let response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": updated_content
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert!(!body["result"]["version_id"].as_str().unwrap().is_empty());

    let read_content = read_file(&app, &workspace_id, &token, "/test.txt").await;
    assert_eq!(read_content.as_str().unwrap(), updated_content);
}

#[tokio::test]
async fn test_write_nested_path() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Nested Test").await;

    let content = "nested content";
    write_file(&app, &workspace_id, &token, "/folder/subfolder/nested.txt", serde_json::json!(content)).await;

    let read_content = read_file(&app, &workspace_id, &token, "/folder/subfolder/nested.txt").await;
    assert_eq!(read_content.as_str().unwrap(), content);
}

#[tokio::test]
async fn test_write_duplicate_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Dedup Test").await;

    let content = "same content";

    let first_write = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": content
    })).await;

    assert_eq!(first_write.status(), 200);
    let first_body: serde_json::Value = first_write.json().await.unwrap();
    let first_version_id = first_body["result"]["version_id"].as_str().unwrap();

    let second_write = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": content
    })).await;

    assert_eq!(second_write.status(), 200);
    let second_body: serde_json::Value = second_write.json().await.unwrap();
    let second_version_id = second_body["result"]["version_id"].as_str().unwrap();

    assert_eq!(first_version_id, second_version_id);
}

#[tokio::test]
async fn test_write_invalid_file_type() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Invalid Type Test").await;

    let content = "content";
    let response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": content,
        "file_type": "invalid_type"
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["error"].as_str().unwrap().contains("Validation failed"));
    assert!(body["fields"]["file_type"].as_str().unwrap().contains("Invalid file type"));
}

#[tokio::test]
async fn test_write_folder() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Folder Test").await;

    let response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/my-folder",
        "content": {},
        "file_type": "folder"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert!(!body["result"]["file_id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_write_markdown_content() {
    // Files can store markdown content
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Markdown Test").await;

    let markdown_content = "# Heading\n\nSome **bold** text.";
    let response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/doc.md",
        "content": markdown_content
    })).await;

    assert_eq!(response.status(), 200);

    // Read back - should return the markdown string
    let read_content = read_file(&app, &workspace_id, &token, "/doc.md").await;
    assert_eq!(read_content.as_str().unwrap(), markdown_content);
}

#[tokio::test]
async fn test_write_multiline_text() {
    // Test multiline text content
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Multiline Test").await;

    let content = "Line 1\nLine 2\nLine 3";
    write_file(&app, &workspace_id, &token, "/multiline.txt", serde_json::json!(content)).await;

    let read_content = read_file(&app, &workspace_id, &token, "/multiline.txt").await;
    assert_eq!(read_content.as_str().unwrap(), content);
}
