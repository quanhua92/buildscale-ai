//! Tests for rm tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_rm_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "RM Test").await;

    let file_id = write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "to delete"})).await;

    let response = execute_tool(&app, &workspace_id, &token, "rm", serde_json::json!({
        "path": "/test.txt"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["file_id"], file_id);

    let read_response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/test.txt"
    })).await;
    assert_eq!(read_response.status(), 404);
}

#[tokio::test]
async fn test_rm_empty_folder() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "RM Folder Test").await;

    // Create folder using POST /files API
    let create_response = app.client
        .post(&format!("{}/api/v1/workspaces/{}/files", app.address, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": "folder",
            "file_type": "folder",
            "content": {}
        }))
        .send()
        .await
        .expect("Failed to create folder");
    assert_eq!(create_response.status(), 200);

    let response = execute_tool(&app, &workspace_id, &token, "rm", serde_json::json!({
        "path": "/folder"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_rm_folder_with_children() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "RM Nonempty Test").await;

    // Create folder using POST /files API
    let create_response = app.client
        .post(&format!("{}/api/v1/workspaces/{}/files", app.address, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": "folder",
            "file_type": "folder",
            "content": {}
        }))
        .send()
        .await
        .expect("Failed to create folder");
    assert_eq!(create_response.status(), 200);

    // Add child file to folder
    let folder_data: serde_json::Value = create_response.json().await.unwrap();
    let folder_id = folder_data["file"]["id"].as_str().unwrap();

    let child_response = app.client
        .post(&format!("{}/api/v1/workspaces/{}/files", app.address, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "parent_id": folder_id,
            "name": "file.txt",
            "file_type": "document",
            "content": {"text": "child"}
        }))
        .send()
        .await
        .expect("Failed to create child file");

    // Verify child file was created successfully
    assert_eq!(child_response.status(), 200, "Child file should be created successfully");

    // Try to delete folder with children - should fail
    let response = execute_tool(&app, &workspace_id, &token, "rm", serde_json::json!({
        "path": "/folder"
    })).await;

    assert_eq!(response.status(), 409);
}

#[tokio::test]
async fn test_rm_nonexistent_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "RM Nonexistent Test").await;

    let response = execute_tool(&app, &workspace_id, &token, "rm", serde_json::json!({
        "path": "/nonexistent.txt"
    })).await;

    assert_eq!(response.status(), 404);
}
