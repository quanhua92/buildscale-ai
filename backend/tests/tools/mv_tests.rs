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
