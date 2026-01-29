use buildscale::{
    load_config,
    models::{
        files::FileType,
        requests::{CreateFileRequest, CreateVersionRequest},
    },
    services::files::{
        create_file_with_content, create_version, get_file_with_content, DEFAULT_FILE_PERMISSION,
        DEFAULT_FOLDER_PERMISSION,
    },
    services::storage::FileStorageService,
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_default_permissions_by_type() {
    let test_app = TestApp::new("test_default_permissions_by_type").await;
    let mut conn = test_app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // 1. Create a Document (should get 600)
    let doc_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "default_doc.md".to_string(),
        slug: None,
        path: None,
        is_virtual: None,
        is_remote: None,
        permission: None, // Omitted
        file_type: FileType::Document,
        content: serde_json::json!({}),
        app_data: None,
    };
    let doc = create_file_with_content(&mut conn, &storage, doc_request)
        .await
        .unwrap();
    assert_eq!(
        doc.file.permission, DEFAULT_FILE_PERMISSION,
        "Document should have default file permission (600)"
    );

    // 2. Create a Folder (should get 755)
    let folder_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "default_folder".to_string(),
        slug: None,
        path: None,
        is_virtual: None,
        is_remote: None,
        permission: None, // Omitted
        file_type: FileType::Folder,
        content: serde_json::json!({}),
        app_data: None,
    };
    let folder = create_file_with_content(&mut conn, &storage, folder_request)
        .await
        .unwrap();
    assert_eq!(
        folder.file.permission, DEFAULT_FOLDER_PERMISSION,
        "Folder should have default folder permission (755)"
    );

    // 3. Override permission (should respect request)
    let override_request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "override.txt".to_string(),
        slug: None,
        path: None,
        is_virtual: None,
        is_remote: None,
        permission: Some(777),
        file_type: FileType::Document,
        content: serde_json::json!({}),
        app_data: None,
    };
    let over = create_file_with_content(&mut conn, &storage, override_request)
        .await
        .unwrap();
    assert_eq!(over.file.permission, 777, "Should respect explicit permission");
}

#[tokio::test]
async fn test_create_file_atomic_success() {
    let test_app = TestApp::new("test_create_file_atomic_success").await;
    let mut conn = test_app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "test_file.md".to_string(),
        slug: None,
        path: None,
        is_virtual: None,
        is_remote: None,
        permission: None,
        file_type: FileType::Document,
        content: serde_json::json!({"text": "hello world"}),
        app_data: None,
    };

    let result = create_file_with_content(&mut conn, &storage, request).await;
    assert!(result.is_ok(), "File creation should succeed");

    let file_with_content = result.unwrap();
    assert_eq!(file_with_content.file.slug, "test_file.md");
    assert_eq!(file_with_content.content["text"], "hello world");
}

#[tokio::test]
async fn test_version_deduplication() {
    let test_app = TestApp::new("test_version_deduplication").await;
    let mut conn = test_app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // 1. Create initial file
    let request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "dedup_test.md".to_string(),
        slug: None,
        path: None,
        is_virtual: None,
        is_remote: None,
        permission: None,
        file_type: FileType::Document,
        content: serde_json::json!({"text": "original"}),
        app_data: None,
    };
    let created = create_file_with_content(&mut conn, &storage, request).await.unwrap();
    let file_id = created.file.id;
    let original_version_id = created.latest_version.id;

    // 2. Try to create version with IDENTICAL content
    let update_request = CreateVersionRequest {
        author_id: Some(user.id),
        branch: None,
        content: serde_json::json!({"text": "original"}),
        app_data: None,
    };
    let updated = create_version(&mut conn, &storage, file_id, update_request).await.unwrap();

    // 3. Verify it's a DIFFERENT version ID (no longer deduplicated)
    assert_ne!(updated.id, original_version_id, "Should return new version even for identical content");
}

#[tokio::test]
async fn test_version_history() {
    let test_app = TestApp::new("test_version_history").await;
    let mut conn = test_app.get_connection().await;
    let storage = FileStorageService::new(&load_config().unwrap().storage.base_path);

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // 1. Create initial file
    let request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "history_test.md".to_string(),
        slug: None,
        path: None,
        is_virtual: None,
        is_remote: None,
        permission: None,
        file_type: FileType::Document,
        content: serde_json::json!({"v": 1}),
        app_data: None,
    };
    let created = create_file_with_content(&mut conn, &storage, request).await.unwrap();
    let file_id = created.file.id;

    // 2. Add v2
    let update_request = CreateVersionRequest {
        author_id: Some(user.id),
        branch: None,
        content: serde_json::json!({"v": 2}),
        app_data: None,
    };
    create_version(&mut conn, &storage, file_id, update_request).await.unwrap();

    // 3. Verify get_file_with_content returns v2
    let fetched = get_file_with_content(&mut conn, &storage, file_id).await.unwrap();
    assert_eq!(fetched.content["v"], 2);
}
