use buildscale::models::{
    files::FileType,
    requests::{AddLinkHttp, AddTagHttp, CreateFileHttp, CreateVersionHttp, UpdateFileHttp, SemanticSearchHttp},
};
use buildscale::services::files::process_file_for_ai;
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
            name: "api_test.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
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
            name: "secret.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
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
            name: "my_folder".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
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
            name: "inside.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
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
            name: "target_folder".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
            file_type: FileType::Folder,
            content: serde_json::json!({}),
            app_data: None,
        }).send().await.unwrap();
    let folder_id = folder_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();
    let folder_uuid = uuid::Uuid::parse_str(&folder_id).unwrap();

    // 2. Create a file in root
    let file_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            name: "move_me.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
            file_type: FileType::Document,
            content: serde_json::json!({"text": "original"}),
            app_data: None,
        }).send().await.unwrap();
    let file_id = file_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();

    // 3. Move file into folder and rename it
    // Use the struct directly to avoid JSON serialization ambiguity
    let patch_resp = app.client.patch(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&UpdateFileHttp {
            parent_id: Some(Some(folder_uuid)),
            name: Some("renamed.md".to_string()),
            slug: None,
            is_virtual: None,
            permission: None,
        }).send().await.unwrap();
    
    assert_eq!(patch_resp.status(), 200);
    let patched_body: serde_json::Value = patch_resp.json().await.unwrap();
    assert_eq!(patched_body["slug"], "renamed.md");
    assert_eq!(patched_body["parent_id"], folder_id);

    // 4. Move file back to root
    let root_move_resp = app.client.patch(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&UpdateFileHttp {
            parent_id: Some(None), // Explicitly move to root
            name: None,
            slug: None,
            is_virtual: None,
            permission: None,
        }).send().await.unwrap();
    
    assert_eq!(root_move_resp.status(), 200);
    let root_body: serde_json::Value = root_move_resp.json().await.unwrap();
    assert!(root_body["parent_id"].is_null(), "Expected parent_id to be null after moving to root, got: {:?}", root_body["parent_id"]);
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
            name: "trash_me.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
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

#[tokio::test]
async fn test_tagging_lifecycle() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Tag WS").await;

    // 1. Create file
    let create_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            name: "tag_me.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
            file_type: FileType::Document,
            content: serde_json::json!({}),
            app_data: None,
        }).send().await.unwrap();
    let file_id = create_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();

    // 2. Add tag
    app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files/{}/tags", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&AddTagHttp { tag: "Research".to_string() })
        .send().await.unwrap();

    // 3. Search by tag (lowercase check)
    let search_resp = app.client.get(&app.url(&format!("/api/v1/workspaces/{}/files/tags/research", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let items: Vec<serde_json::Value> = search_resp.json().await.unwrap();
    assert!(items.iter().any(|i| i["id"] == file_id));

    // 4. Remove tag
    app.client.delete(&app.url(&format!("/api/v1/workspaces/{}/files/{}/tags/research", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();

    // 5. Verify gone
    let final_search = app.client.get(&app.url(&format!("/api/v1/workspaces/{}/files/tags/research", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let final_items: Vec<serde_json::Value> = final_search.json().await.unwrap();
    assert!(final_items.is_empty());
}

#[tokio::test]
async fn test_backlink_discovery() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Link WS").await;

    // 1. Create File A
    let a_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            name: "file_a.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
            file_type: FileType::Document,
            content: serde_json::json!({}),
            app_data: None,
        }).send().await.unwrap();
    let file_a_id = a_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();

    // 2. Create File B
    let b_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            name: "file_b.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
            file_type: FileType::Document,
            content: serde_json::json!({}),
            app_data: None,
        }).send().await.unwrap();
    let file_b_id = b_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();

    // 3. Link A -> B
    app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files/{}/links", workspace_id, file_a_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&AddLinkHttp { target_file_id: uuid::Uuid::parse_str(&file_b_id).unwrap() })
        .send().await.unwrap();

    // 4. Check B's network (backlinks)
    let network_resp = app.client.get(&app.url(&format!("/api/v1/workspaces/{}/files/{}/network", workspace_id, file_b_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let network: serde_json::Value = network_resp.json().await.unwrap();
    
    assert!(network["backlinks"].as_array().unwrap().iter().any(|f| f["id"] == file_a_id));
}

#[tokio::test]
async fn test_semantic_search_flow() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "AI WS").await;
    let mut conn = app.get_connection().await;

    // 1. Create a file with content
    let create_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            name: "ai_doc.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
            file_type: FileType::Document,
            content: serde_json::json!("This is a document about machine learning and artificial intelligence."),
            app_data: None,
        }).send().await.unwrap();
    let file_id_str = create_resp.json::<serde_json::Value>().await.unwrap()["file"]["id"].as_str().unwrap().to_string();
    let file_id = uuid::Uuid::parse_str(&file_id_str).unwrap();

    // 2. Trigger AI ingestion (manually for test)
    let ai_config = buildscale::config::AiConfig::default();
    process_file_for_ai(&mut conn, file_id, &ai_config).await.expect("AI ingestion failed");

    // 3. Verify status is Ready
    let get_resp = app.client.get(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let file_body: serde_json::Value = get_resp.json().await.unwrap();
    assert_eq!(file_body["file"]["status"], "ready");

    // 4. Perform search
    // Since we used dummy vectors [0.1; 1536], searching with any vector will return it
    let search_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/search", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&SemanticSearchHttp {
            query_vector: vec![0.1; 1536],
            limit: Some(5),
        }).send().await.unwrap();
    
    assert_eq!(search_resp.status(), 200);
    let results: Vec<serde_json::Value> = search_resp.json().await.unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0]["file"]["id"], file_id_str);
    assert!(results[0]["chunk_content"].as_str().unwrap().contains("machine learning"));
}

#[tokio::test]
async fn test_slug_normalization() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Normalization WS").await;

    // 1. Create a file with mixed case and spaces in NAME
    let create_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            name: " My Document.MD ".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
            file_type: FileType::Document,
            content: serde_json::json!({}),
            app_data: None,
        }).send().await.unwrap();

    assert_eq!(create_resp.status(), 200);
    let created_body: serde_json::Value = create_resp.json().await.unwrap();
    assert_eq!(created_body["file"]["name"], "My Document.MD");
    assert_eq!(created_body["file"]["slug"], "my-document.md");
    let file_id = created_body["file"]["id"].as_str().unwrap();

    // 2. Try to create a collision with different case in NAME (resulting in same SLUG)
    let collision_resp = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&CreateFileHttp {
            parent_id: None,
            name: "my document.md".to_string(),
            slug: None,
            path: None,
            is_virtual: None,
            permission: None,
            file_type: FileType::Document,
            content: serde_json::json!({}),
            app_data: None,
        }).send().await.unwrap();

    assert_eq!(collision_resp.status(), 409); // Conflict

    // 3. Rename with normalization
    let rename_resp = app.client.patch(&app.url(&format!("/api/v1/workspaces/{}/files/{}", workspace_id, file_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&UpdateFileHttp {
            parent_id: None,
            name: Some(" RENAMED DOC.md ".to_string()),
            slug: None,
            is_virtual: None,
            permission: None,
        }).send().await.unwrap();

    assert_eq!(rename_resp.status(), 200);
    let renamed_body: serde_json::Value = rename_resp.json().await.unwrap();
    assert_eq!(renamed_body["name"], "RENAMED DOC.md");
    assert_eq!(renamed_body["slug"], "renamed-doc.md");
}
