//! Tests for edit-many tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file, read_file};

#[tokio::test]
async fn test_edit_many_success() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Many Success Test").await;
    
    // 1. Create initial file with multiple occurrences
    let initial_text = "foo bar foo\nbaz foo";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": initial_text})).await;

    // 2. Perform edit-many
    let response = execute_tool(&app, &workspace_id, &token, "edit-many", serde_json::json!({
        "path": "/test.txt",
        "old_string": "foo",
        "new_string": "qux"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    
    // 3. Verify content
    let read_content = read_file(&app, &workspace_id, &token, "/test.txt").await;
    let expected_text = "qux bar qux\nbaz qux";
    assert_eq!(read_content.as_str().unwrap(), expected_text);
}

#[tokio::test]
async fn test_edit_many_not_found() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Many Not Found Test").await;
    
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "content"})).await;

    let response = execute_tool(&app, &workspace_id, &token, "edit-many", serde_json::json!({
        "path": "/test.txt",
        "old_string": "non-existent",
        "new_string": "replacement"
    })).await;

    assert_eq!(response.status(), 400); // Validation error
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["old_string"].as_str().unwrap().contains("Search string not found"));
}

#[tokio::test]
async fn test_edit_many_stale_hash() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Many Stale Hash Test").await;
    
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "initial"})).await;

    let response = execute_tool(&app, &workspace_id, &token, "edit-many", serde_json::json!({
        "path": "/test.txt",
        "old_string": "initial",
        "new_string": "updated",
        "last_read_hash": "wrong-hash"
    })).await;

    assert_eq!(response.status(), 409); // Conflict
}
