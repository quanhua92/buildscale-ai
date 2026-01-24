//! Tests for write tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file, read_file};

#[tokio::test]
async fn test_write_new_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Test").await;
    
    let content = serde_json::json!({"text": "new file content"});
    let file_id = write_file(&app, &workspace_id, &token, "/new.txt", content.clone()).await;
    
    assert!(!file_id.is_empty());
    
    let read_content = read_file(&app, &workspace_id, &token, "/new.txt").await;
    assert_eq!(read_content, content);
}

#[tokio::test]
async fn test_write_update_existing_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Update Test").await;
    
    let initial_content = serde_json::json!({"text": "initial"});
    write_file(&app, &workspace_id, &token, "/test.txt", initial_content).await;
    
    let updated_content = serde_json::json!({"text": "updated"});
    let response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": updated_content.clone()
    })).await;
    
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert!(!body["result"]["version_id"].as_str().unwrap().is_empty());
    
    let read_content = read_file(&app, &workspace_id, &token, "/test.txt").await;
    assert_eq!(read_content, updated_content);
}

#[tokio::test]
async fn test_write_nested_path() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Nested Test").await;
    
    let content = serde_json::json!({"text": "nested content"});
    write_file(&app, &workspace_id, &token, "/folder/subfolder/nested.txt", content.clone()).await;
    
    let read_content = read_file(&app, &workspace_id, &token, "/folder/subfolder/nested.txt").await;
    assert_eq!(read_content, content);
}

#[tokio::test]
async fn test_write_duplicate_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Dedup Test").await;
    
    let content = serde_json::json!({"text": "same content"});
    
    let first_write = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": content.clone()
    })).await;
    
    assert_eq!(first_write.status(), 200);
    let first_body: serde_json::Value = first_write.json().await.unwrap();
    let first_version_id = first_body["result"]["version_id"].as_str().unwrap();
    
    let second_write = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": content.clone()
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
    
    let content = serde_json::json!({"text": "content"});
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
async fn test_write_update_document_invalid_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Update Document Validation Test").await;
    
    // 1. Create a valid document
    let content = serde_json::json!({"text": "valid"});
    write_file(&app, &workspace_id, &token, "/doc.txt", content).await;
    
    // 2. Try to update with invalid content (missing 'text' field)
    let response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/doc.txt",
        "content": {"not_text": "invalid"}
    })).await;
    
    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["content"].as_str().unwrap().contains("Document content must contain a 'text' field"));

    // 3. Try to update with invalid content ('text' field is not a string)
    let response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/doc.txt",
        "content": {"text": 123}
    })).await;
    
    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["content"].as_str().unwrap().contains("Document content must contain a 'text' field with a string value"));
}
