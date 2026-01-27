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

#[tokio::test]
async fn test_rm_strict_isolation() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "RM Isolation").await;

    // 1. Create a nested structure
    for i in 1..=5 {
        let path = format!("/work/file-{}.txt", i);
        write_file(&app, &workspace_id, &token, &path, serde_json::json!({"text": "data"})).await;
    }

    // 2. Delete file-3
    let response = execute_tool(&app, &workspace_id, &token, "rm", serde_json::json!({
        "path": "/work/file-3.txt"
    })).await;
    assert!(response.status().is_success());

    // 3. Verify exactly 4 files remain in /work
    let list_response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/work"
    })).await;
    let body: serde_json::Value = list_response.json().await.unwrap();
    let entries = body["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 4, "Expected exactly 4 files to remain");
    
    // Ensure file-3 is the one missing
    for entry in entries {
        assert_ne!(entry["path"], "/work/file-3.txt");
    }
}

#[tokio::test]
async fn test_rm_folder_protection_with_orphans() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "RM Orphan Test").await;
    
    let me_res = app.client
        .get(&format!("{}/api/v1/auth/me", app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let user_id = uuid::Uuid::parse_str(me_res["user"]["id"].as_str().unwrap()).unwrap();

    // 1. Create a folder "/chats"
    app.client
        .post(&format!("{}/api/v1/workspaces/{}/files", app.address, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": "chats",
            "file_type": "folder",
            "content": {}
        }))
        .send()
        .await
        .expect("Failed to create folder");

    // 2. Create an "ORPHAN" file (parent_id: None, but path: "/chats/...")
    let mut conn = app.pool.acquire().await.unwrap();
    let chat_uuid = uuid::Uuid::now_v7();
    buildscale::queries::files::create_file_identity(&mut conn, buildscale::models::files::NewFile {
        workspace_id: uuid::Uuid::parse_str(&workspace_id).unwrap(),
        parent_id: None, 
        author_id: user_id,
        file_type: buildscale::models::files::FileType::Chat,
        status: buildscale::models::files::FileStatus::Ready,
        name: "My Orphan Chat".to_string(),
        slug: format!("chat-{}", chat_uuid),
        path: format!("/chats/chat-{}", chat_uuid),
        is_virtual: true,
        is_remote: false,
        permission: 600,
    }).await.unwrap();

    // 3. Try to delete the folder /chats - should fail due to logical descendant
    let rm_response = execute_tool(&app, &workspace_id, &token, "rm", serde_json::json!({
        "path": "/chats"
    })).await;

    assert_eq!(rm_response.status(), 409, "Should fail with Conflict (logical descendant)");
}
