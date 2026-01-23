use buildscale::{
    models::{
        files::FileType,
        requests::{CreateFileRequest, CreateVersionRequest},
    },
    services::files::{create_file_with_content, create_version, get_file_with_content},
};
use crate::common::database::TestApp;

#[tokio::test]
async fn test_create_file_atomic_success() {
    let test_app = TestApp::new("test_create_file_atomic_success").await;
    let mut conn = test_app.get_connection().await;

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    let request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "test_file.md".to_string(),
        slug: None,
        path: None,
        file_type: FileType::Document,
        content: serde_json::json!({"text": "hello world"}),
        app_data: None,
    };

    let result = create_file_with_content(&mut conn, request).await;
    assert!(result.is_ok(), "File creation should succeed");

    let file_with_content = result.unwrap();
    assert_eq!(file_with_content.file.slug, "test_file.md");
    assert_eq!(file_with_content.latest_version.content_raw["text"], "hello world");
}

#[tokio::test]
async fn test_version_deduplication() {
    let test_app = TestApp::new("test_version_deduplication").await;
    let mut conn = test_app.get_connection().await;

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // 1. Create initial file
    let request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "dedup_test.md".to_string(),
        slug: None,
        path: None,
        file_type: FileType::Document,
        content: serde_json::json!({"text": "original"}),
        app_data: None,
    };
    let created = create_file_with_content(&mut conn, request).await.unwrap();
    let file_id = created.file.id;
    let original_version_id = created.latest_version.id;

    // 2. Try to create version with IDENTICAL content
    let update_request = CreateVersionRequest {
        author_id: Some(user.id),
        branch: None,
        content: serde_json::json!({"text": "original"}),
        app_data: None,
    };
    let updated = create_version(&mut conn, file_id, update_request).await.unwrap();

    // 3. Verify it's the SAME version ID (deduplicated)
    assert_eq!(updated.id, original_version_id, "Should return existing version for identical content");
}

#[tokio::test]
async fn test_version_history() {
    let test_app = TestApp::new("test_version_history").await;
    let mut conn = test_app.get_connection().await;

    let (user, workspace) = test_app.create_test_workspace_with_user().await.unwrap();

    // 1. Create initial file
    let request = CreateFileRequest {
        workspace_id: workspace.id,
        parent_id: None,
        author_id: user.id,
        name: "history_test.md".to_string(),
        slug: None,
        path: None,
        file_type: FileType::Document,
        content: serde_json::json!({"v": 1}),
        app_data: None,
    };
    let created = create_file_with_content(&mut conn, request).await.unwrap();
    let file_id = created.file.id;

    // 2. Add v2
    let update_request = CreateVersionRequest {
        author_id: Some(user.id),
        branch: None,
        content: serde_json::json!({"v": 2}),
        app_data: None,
    };
    create_version(&mut conn, file_id, update_request).await.unwrap();

    // 3. Verify get_file_with_content returns v2
    let fetched = get_file_with_content(&mut conn, file_id).await.unwrap();
    assert_eq!(fetched.latest_version.content_raw["v"], 2);
}
