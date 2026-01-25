//! Tests for write tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file, read_file, write_file_with_type};

#[tokio::test]
async fn test_write_new_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Test").await;
    
    let content = "new file content";  // Auto-unwrapped for Documents
    let file_id = write_file(&app, &workspace_id, &token, "/new.txt", serde_json::json!({"text": content})).await;

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
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": initial_content})).await;

    let updated_content = "updated";
    let response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": serde_json::json!({"text": updated_content})
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
    
    let content = "nested content";  // Auto-unwrapped for Documents
    write_file(&app, &workspace_id, &token, "/folder/subfolder/nested.txt", serde_json::json!({"text": content})).await;

    let read_content = read_file(&app, &workspace_id, &token, "/folder/subfolder/nested.txt").await;
    assert_eq!(read_content.as_str().unwrap(), content);
}

#[tokio::test]
async fn test_write_duplicate_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Write Dedup Test").await;
    
    let content = "same content";  // Auto-unwrapped for Documents
    
    let first_write = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": serde_json::json!({"text": content})
    })).await;

    assert_eq!(first_write.status(), 200);
    let first_body: serde_json::Value = first_write.json().await.unwrap();
    let first_version_id = first_body["result"]["version_id"].as_str().unwrap();

    let second_write = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.txt",
        "content": serde_json::json!({"text": content})
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

#[tokio::test]
async fn test_write_auto_wrap_string() {
    // Verify that raw strings are auto-wrapped and then auto-unwrapped for Documents
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Auto Wrap Test").await;

    // Write raw string (should be auto-wrapped to {"text": "raw string content"})
    let response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/auto.txt",
        "content": "raw string content"  // Not wrapped - raw string
    })).await;

    assert_eq!(response.status(), 200);

    // Read back (should be auto-unwrapped to just the string)
    let read_content = read_file(&app, &workspace_id, &token, "/auto.txt").await;
    assert_eq!(read_content.as_str().unwrap(), "raw string content");
}

#[tokio::test]
async fn test_write_canvas_preserves_jsonb() {
    // Canvas files preserve JSON structure without auto-wrap/unwrap
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Canvas Test").await;

    let canvas_content = serde_json::json!({
        "elements": [{"type": "rect", "x": 10, "y": 20}],
        "metadata": {"version": 1}
    });

    write_file_with_type(&app, &workspace_id, &token, "/canvas.json", canvas_content.clone(), "canvas").await;

    // Read back - should be identical (no unwrap for non-Document types)
    let read_content = read_file(&app, &workspace_id, &token, "/canvas.json").await;
    assert_eq!(read_content, canvas_content);
}

