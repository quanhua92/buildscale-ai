use buildscale::models::{
    files::FileType,
    requests::{CreateFileHttp, CreateVersionHttp},
};
use crate::common::{TestApp, TestAppOptions, register_and_login};

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
