use buildscale::models::{
    files::FileType,
    requests::{CreateFileHttp, CreateVersionHttp, UpdateFileHttp},
};
use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};

#[tokio::test]
async fn test_file_api_lifecycle() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;

    // 1. Create a workspace
    let ws_resp = app
        .client
        .post(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": "File Test Workspace"
        }))
        .send()
        .await
        .unwrap();
    let ws_body: serde_json::Value = ws_resp.json().await.unwrap();
    let workspace_id = ws_body["workspace"]["id"].as_str().unwrap();

    // 2. Create a file
    let create_resp = app
        .client
        .post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            slug: "api_test.md".to_string(),
            file_type: FileType::Document,
            content: serde_json::json!({"text": "initial content"}),
            app_data: None,
        })
        .send()
        .await
        .unwrap();

    assert_eq!(create_resp.status(), 200);
    let created_body: serde_json::Value = create_resp.json().await.unwrap();
    let file_id = created_body["file"]["id"].as_str().unwrap();
    assert_eq!(created_body["file"]["slug"], "api_test.md");
    assert_eq!(created_body["latest_version"]["content_raw"]["text"], "initial content");

    // 3. Get the file
    let get_resp = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(get_resp.status(), 200);
    let fetched_body: serde_json::Value = get_resp.json().await.unwrap();
    assert_eq!(fetched_body["file"]["id"], file_id);

    // 4. Create a new version
    let version_resp = app
        .client
        .post(&app.url(&format!("/api/v1/workspaces/{}/files/{}/versions", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateVersionHttp {
            branch: None,
            content: serde_json::json!({"text": "updated content"}),
            app_data: None,
        })
        .send()
        .await
        .unwrap();

    assert_eq!(version_resp.status(), 200);
    let version_body: serde_json::Value = version_resp.json().await.unwrap();
    assert_eq!(version_body["content_raw"]["text"], "updated content");

    // 5. Verify latest version is updated
    let final_resp = app
        .client
        .get(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    let final_body: serde_json::Value = final_resp.json().await.unwrap();
    assert_eq!(final_body["latest_version"]["content_raw"]["text"], "updated content");
}

#[tokio::test]
async fn test_file_api_permission_denied() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    
    // User 1 creates workspace and file
    let token1 = register_and_login(&app).await;
    let ws_resp = app.client.post(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", token1))
        .json(&serde_json::json!({"name": "WS1"}))
        .send().await.unwrap();
    let workspace_id = ws_resp.json::<serde_json::Value>().await.unwrap()["workspace"]["id"].as_str().unwrap().to_string();

    let create_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token1))
        .json(&CreateFileHttp {
            parent_id: None,
            slug: "secret.md".to_string(),
            file_type: FileType::Document,
            content: serde_json::json!({"text": "secret"}),
            app_data: None,
        }).send().await.unwrap();
    let file_id = create_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();

    // User 2 tries to access User 1's file
    let token2 = register_and_login(&app).await;
    let get_resp = app.client.get(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token2))
        .send().await.unwrap();

    assert_eq!(get_resp.status(), 403);
}

#[tokio::test]
async fn test_folder_delete_safeguard() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Delete Test WS").await;

    // 1. Create a folder
    let folder_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            slug: "my_folder".to_string(),
            file_type: FileType::Folder,
            content: serde_json::json!({}),
            app_data: None,
        }).send().await.unwrap();
    let folder_id = folder_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();

    // 2. Create a file inside that folder
    app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: Some(uuid::Uuid::parse_str(&folder_id).unwrap()),
            slug: "inside.md".to_string(),
            file_type: FileType::Document,
            content: serde_json::json!({"text": "hello"}),
            app_data: None,
        }).send().await.unwrap();

    // 3. Try to delete the folder - should fail (409 Conflict)
    let delete_resp = app.client.delete(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, folder_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    
    assert_eq!(delete_resp.status(), 409);

    // 4. Delete the file first
    let list_resp = app.client.get(&app.url(&format!("/api/v1/workspaces/{}/files/trash", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(list_resp.status(), 200);
}

#[tokio::test]
async fn test_move_rename_lifecycle() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Move Test WS").await;

    // 1. Create a folder
    let folder_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            slug: "target_folder".to_string(),
            file_type: FileType::Folder,
            content: serde_json::json!({}),
            app_data: None,
        }).send().await.unwrap();
    let folder_id = folder_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();

    // 2. Create a file in root
    let file_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            slug: "move_me.md".to_string(),
            file_type: FileType::Document,
            content: serde_json::json!({"text": "original"}),
            app_data: None,
        }).send().await.unwrap();
    let file_id = file_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();

    // 3. Move file into folder and rename it
    let patch_resp = app.client.patch(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&UpdateFileHttp {
            parent_id: Some(uuid::Uuid::parse_str(&folder_id).unwrap()),
            slug: Some("renamed.md".to_string()),
        }).send().await.unwrap();
    
    assert_eq!(patch_resp.status(), 200);
    let patched_body: serde_json::Value = patch_resp.json().await.unwrap();
    assert_eq!(patched_body["slug"], "renamed.md");
    assert_eq!(patched_body["parent_id"], folder_id);
}

#[tokio::test]
async fn test_trash_restore_lifecycle() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Trash Test WS").await;

    // 1. Create file
    let create_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            slug: "trash_me.md".to_string(),
            file_type: FileType::Document,
            content: serde_json::json!({}),
            app_data: None,
        }).send().await.unwrap();
    let file_id = create_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();

    // 2. Delete it
    let delete_resp = app.client.delete(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(delete_resp.status(), 200);

    // 3. Check trash
    let trash_resp = app.client.get(&app.url(&format!("/api/v1/workspaces/{}/files/trash", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let trash_items: Vec<serde_json::Value> = trash_resp.json().await.unwrap();
    assert!(trash_items.iter().any(|i| i["id"] == file_id));

    // 4. Restore it
    let restore_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files/{}/restore", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(restore_resp.status(), 200);

    // 5. Verify it's back in root
    let final_resp = app.client.get(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(final_resp.status(), 200);
}
