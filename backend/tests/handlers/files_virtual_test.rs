use buildscale::models::{
    files::FileType,
    requests::{CreateFileHttp, UpdateFileHttp},
};
use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};

#[tokio::test]
async fn test_virtual_file_lifecycle() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Virtual WS").await;

    // 1. Create a virtual file
    let create_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            name: "chat_log.chat".to_string(),
            slug: None,
            path: None,
            is_virtual: Some(true),
            is_remote: Some(false),
            permission: Some(600),
            file_type: FileType::Chat,
            content: serde_json::json!({}), // Virtual files might have empty initial content
            app_data: None,
        })
        .send()
        .await
        .unwrap();

    assert_eq!(create_resp.status(), 200);
    let created_body: serde_json::Value = create_resp.json().await.unwrap();
    let file_id = created_body["file"]["id"].as_str().unwrap();
    
    assert_eq!(created_body["file"]["is_virtual"], true);
    assert_eq!(created_body["file"]["permission"], 600);

    // 2. Get the file to verify persistence
    let get_resp = app.client.get(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(get_resp.status(), 200);
    let fetched_body: serde_json::Value = get_resp.json().await.unwrap();
    assert_eq!(fetched_body["file"]["is_virtual"], true);
    assert_eq!(fetched_body["file"]["permission"], 600);

    // 3. Update permission to 755 (public read)
    let patch_resp = app.client.patch(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&UpdateFileHttp {
            parent_id: None,
            name: None,
            slug: None,
            is_virtual: None, // Should remain true
            is_remote: None,
            permission: Some(755),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(patch_resp.status(), 200);
    let patched_body: serde_json::Value = patch_resp.json().await.unwrap();
    assert_eq!(patched_body["permission"], 755);
    assert_eq!(patched_body["is_virtual"], true); // Should persist

    // 4. Update is_virtual to false (convert to real file)
    let patch_virtual_resp = app.client.patch(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&UpdateFileHttp {
            parent_id: None,
            name: None,
            slug: None,
            is_virtual: Some(false),
            is_remote: None,
            permission: None,
        })
        .send()
        .await
        .unwrap();
    
    assert_eq!(patch_virtual_resp.status(), 200);
    let final_body: serde_json::Value = patch_virtual_resp.json().await.unwrap();
    assert_eq!(final_body["is_virtual"], false);
    assert_eq!(final_body["permission"], 755); // Should persist
}
