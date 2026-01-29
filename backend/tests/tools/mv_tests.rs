//! Tests for mv tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_mv_rename_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "MV Rename Test").await;
    
    write_file(&app, &workspace_id, &token, "/old.txt", serde_json::json!({"text": "rename me"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "mv", serde_json::json!({
        "source": "/old.txt",
        "destination": "/new.txt"
    })).await;
    
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["to_path"], "/new.txt");
    
    // Verify old path is gone
    let read_old = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({"path": "/old.txt"})).await;
    assert_eq!(read_old.status(), 404);
    
    // Verify new path exists
    let read_new = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({"path": "/new.txt"})).await;
    assert_eq!(read_new.status(), 200);
}

#[tokio::test]
async fn test_mv_move_to_folder() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "MV Folder Test").await;
    
    // Create folder
    execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/target-dir",
        "file_type": "folder",
        "content": {}
    })).await;
    
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!({"text": "move me"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "mv", serde_json::json!({
        "source": "/file.txt",
        "destination": "/target-dir/"
    })).await;
    
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["to_path"], "/target-dir/file.txt");
}

#[tokio::test]
async fn test_mv_conflict_existing_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "MV Conflict Test").await;

    // Create two files
    write_file(&app, &workspace_id, &token, "/file1.txt", serde_json::json!({"text": "file 1"})).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!({"text": "file 2"})).await;

    // Try to rename file1.txt to file2.txt (which already exists)
    let response = execute_tool(&app, &workspace_id, &token, "mv", serde_json::json!({
        "source": "/file1.txt",
        "destination": "/file2.txt"
    })).await;

    // Should return 409 Conflict
    assert_eq!(response.status(), 409);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "CONFLICT");
    assert!(body["error"].as_str().unwrap().contains("already exists"));
}

#[tokio::test]
async fn test_mv_to_root() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "MV Root Test").await;
    
    // 1. Create a file in a subfolder
    execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/folder/sub/file.txt",
        "content": "move me to root"
    })).await;
    
    // 2. Move it to root "/"
    let response = execute_tool(&app, &workspace_id, &token, "mv", serde_json::json!({
        "source": "/folder/sub/file.txt",
        "destination": "/"
    })).await;
    
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["to_path"], "/file.txt");
    
    // 3. Verify it exists at root
    let read_root = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({"path": "/file.txt"})).await;
    assert_eq!(read_root.status(), 200);
}

#[tokio::test]
async fn test_mv_folder_safety() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "MV Safety Test").await;
    
    // Create nested folder structure: /parent/child
    execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/parent/child",
        "file_type": "folder",
        "content": {}
    })).await;
    
    // 1. Try to move /parent into /parent/ (into itself)
    let resp1 = execute_tool(&app, &workspace_id, &token, "mv", serde_json::json!({
        "source": "/parent",
        "destination": "/parent/"
    })).await;
    
    assert_eq!(resp1.status(), 400); // Validation error
    let body1: serde_json::Value = resp1.json().await.unwrap();
    assert_eq!(body1["code"], "VALIDATION_ERROR");
    assert!(body1["fields"]["destination"].as_str().unwrap().contains("itself or a subfolder"));
    
    // 2. Try to move /parent into /parent/child/ (into subfolder)
    let resp2 = execute_tool(&app, &workspace_id, &token, "mv", serde_json::json!({
        "source": "/parent",
        "destination": "/parent/child/"
    })).await;
    
    assert_eq!(resp2.status(), 400);
    let body2: serde_json::Value = resp2.json().await.unwrap();
    assert_eq!(body2["code"], "VALIDATION_ERROR");
    assert!(body2["fields"]["destination"].as_str().unwrap().contains("itself or a subfolder"));
}
