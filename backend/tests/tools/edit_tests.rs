//! Tests for edit tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file, read_file};

#[tokio::test]
async fn test_edit_success() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Success Test").await;
    
    // 1. Create initial file
    let initial_text = "Hello world!\nThis is a test file.\nBuildScale is awesome.";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": initial_text})).await;

    // 2. Perform edit
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "old_string": "BuildScale is awesome.",
        "new_string": "BuildScale is the future."
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    
    // 3. Verify content
    let read_content = read_file(&app, &workspace_id, &token, "/test.txt").await;
    let expected_text = "Hello world!\nThis is a test file.\nBuildScale is the future.";
    assert_eq!(read_content.as_str().unwrap(), expected_text);
}

#[tokio::test]
async fn test_edit_not_found() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Not Found Test").await;
    
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "content"})).await;

    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "old_string": "non-existent string",
        "new_string": "replacement"
    })).await;

    assert_eq!(response.status(), 400); // Validation error
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["old_string"].as_str().unwrap().contains("Search string not found"));
}

#[tokio::test]
async fn test_edit_multiple_matches() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Multi Match Test").await;
    
    let text = "repeat repeat repeat";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": text})).await;

    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "old_string": "repeat",
        "new_string": "single"
    })).await;

    assert_eq!(response.status(), 400); // Validation error
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["old_string"].as_str().unwrap().contains("found 3 times"));
}

#[tokio::test]
async fn test_edit_empty_old_string() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Empty Test").await;
    
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "content"})).await;

    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "old_string": "",
        "new_string": "replacement"
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_edit_wrong_file_type() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Folder Test").await;
    
    // Create a folder
    execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/my-folder",
        "content": {},
        "file_type": "folder"
    })).await;

    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/my-folder",
        "old_string": "anything",
        "new_string": "replacement"
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["path"].as_str().unwrap().contains("only supports Document"));
}

#[tokio::test]
async fn test_edit_stale_hash() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Stale Hash Test").await;
    
    // 1. Create file
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "initial content"})).await;

    // 2. Get the hash (by reading it)
    let _response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({"path": "/test.txt"})).await;
    // We need to get the hash from the file system or response if we exposed it. 
    // Wait, the 'read' tool result doesn't return the hash. 
    // Let's check 'read' result in models/requests.rs or TOOLS_API_GUIDE.md
    
    // Actually, I'll just use a dummy hash first to see it fail.
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "old_string": "initial",
        "new_string": "updated",
        "last_read_hash": "wrong-hash"
    })).await;

    assert_eq!(response.status(), 409); // Conflict
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "CONFLICT");
    assert!(body["error"].as_str().unwrap().contains("File content has changed"));
}

#[tokio::test]
async fn test_edit_correct_hash() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Correct Hash Test").await;
    
    // 1. Create file
    let initial_write = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": "initial content"
    })).await;
    let write_body: serde_json::Value = initial_write.json().await.unwrap();
    let hash = write_body["result"]["hash"].as_str().unwrap().to_string();

    // 2. Edit with correct hash
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "old_string": "initial",
        "new_string": "updated",
        "last_read_hash": hash
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    
    let read_content = read_file(&app, &workspace_id, &token, "/test.txt").await;
    assert_eq!(read_content.as_str().unwrap(), "updated content");
}
