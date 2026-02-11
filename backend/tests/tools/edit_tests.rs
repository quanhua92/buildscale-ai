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
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(initial_text)).await;

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

    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!("content")).await;

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
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(text)).await;

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

    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!("content")).await;

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
    assert!(body["fields"]["path"].as_str().unwrap().contains("Cannot edit a folder"));
}

#[tokio::test]
async fn test_edit_stale_hash() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Stale Hash Test").await;

    // 1. Create file
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!("initial content")).await;

    // 2. Try to edit with wrong hash
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

#[tokio::test]
async fn test_edit_raw_string_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Raw Test").await;
    
    // Create a file with raw string content
    let res = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": "raw.txt",
            "file_type": "document",
            "content": "raw text content"
        }))
        .send().await.unwrap();
    assert!(res.status().is_success());

    // Perform edit on raw content
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/raw.txt",
        "old_string": "text",
        "new_string": "updated"
    })).await;

    assert_eq!(response.status(), 200);
    
    let read_content = read_file(&app, &workspace_id, &token, "/raw.txt").await;
    assert_eq!(read_content.as_str().unwrap(), "raw updated content");
}

// ============================================================================
// INSERT OPERATION TESTS
// ============================================================================

#[tokio::test]
async fn test_edit_insert_at_beginning() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Insert Beginning Test").await;

    let initial_content = "Line 1\nLine 2\nLine 3";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(initial_content)).await;

    // Insert at line 0 (beginning)
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "insert_line": 0,
        "insert_content": "First line!"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    // Verify content
    let read_content = read_file(&app, &workspace_id, &token, "/test.txt").await;
    let expected = "First line!\nLine 1\nLine 2\nLine 3";
    assert_eq!(read_content.as_str().unwrap(), expected);
}

#[tokio::test]
async fn test_edit_insert_at_middle() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Insert Middle Test").await;

    let initial_content = "Line 1\nLine 2\nLine 3";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(initial_content)).await;

    // Insert at line 1 (middle)
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "insert_line": 1,
        "insert_content": "Inserted line"
    })).await;

    assert_eq!(response.status(), 200);

    let read_content = read_file(&app, &workspace_id, &token, "/test.txt").await;
    let expected = "Line 1\nInserted line\nLine 2\nLine 3";
    assert_eq!(read_content.as_str().unwrap(), expected);
}

#[tokio::test]
async fn test_edit_insert_at_end() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Insert End Test").await;

    let initial_content = "Line 1\nLine 2\nLine 3";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(initial_content)).await;

    // Insert at line 3 (end - after last line)
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "insert_line": 3,
        "insert_content": "Last line"
    })).await;

    assert_eq!(response.status(), 200);

    let read_content = read_file(&app, &workspace_id, &token, "/test.txt").await;
    let expected = "Line 1\nLine 2\nLine 3\nLast line";
    assert_eq!(read_content.as_str().unwrap(), expected);
}

#[tokio::test]
async fn test_edit_insert_out_of_bounds() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Insert Out of Bounds Test").await;

    let initial_content = "Line 1\nLine 2";
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(initial_content)).await;

    // Try to insert at line 100 (out of bounds)
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "insert_line": 100,
        "insert_content": "Out of bounds"
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["insert_line"].as_str().unwrap().contains("out of bounds"));
}

#[tokio::test]
async fn test_edit_insert_empty_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Insert Empty Test").await;

    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!("content")).await;

    // Try to insert empty content
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "insert_line": 1,
        "insert_content": ""
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_edit_insert_requires_both_params() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Insert Missing Params Test").await;

    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!("content")).await;

    // Try to insert without insert_content
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "insert_line": 1
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["operation"].as_str().unwrap().contains("Must specify"));
}

#[tokio::test]
async fn test_edit_insert_and_replace_mutually_exclusive() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Insert Replace Exclusive Test").await;

    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!("content")).await;

    // Try to specify both replace and insert params
    let response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": "/test.txt",
        "old_string": "old",
        "new_string": "new",
        "insert_line": 1,
        "insert_content": "inserted"
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["operation"].as_str().unwrap().contains("Cannot specify both"));
}
