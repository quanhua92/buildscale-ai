use buildscale::models::files::{FileType, NewFile, NewFileVersion};
use buildscale::queries::files;
use buildscale::services::storage::FileStorageService;
use buildscale::workers::archive_cleanup::process_cleanup_batch;
use crate::common::{TestApp, TestAppOptions, register_and_login};
use uuid::Uuid;

#[tokio::test]
async fn test_archive_cleanup_worker() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&app.config.storage.base_path);
    
    // 1. Get current user ID
    let token = register_and_login(&app).await;
    let user_info = app.client.get(&app.url("/api/v1/auth/me"))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    let user_id = Uuid::parse_str(user_info["user"]["id"].as_str().unwrap()).unwrap();

    // 2. Create a workspace
    let workspace_id = Uuid::now_v7();
    sqlx::query!("INSERT INTO workspaces (id, name, owner_id) VALUES ($1, $2, $3)", 
        workspace_id, "Cleanup Test", user_id)
        .execute(&mut *conn).await.unwrap();

    // 3. Create a file
    let new_file = NewFile {
        workspace_id,
        parent_id: None,
        author_id: user_id,
        file_type: FileType::Document,
        status: buildscale::models::files::FileStatus::Ready,
        name: "cleanup_test.txt".to_string(),
        slug: "cleanup_test.txt".to_string(),
        path: "/cleanup_test.txt".to_string(),
        is_virtual: false,
        is_remote: false,
        permission: 600,
    };
    let file = files::create_file_identity(&mut *conn, new_file).await.unwrap();
    let file_id = file.id;

    // 4. Create a version (double write will create archive blob)
    let content = b"content to be deleted";
    let hash = storage.write_file(workspace_id, "/cleanup_test.txt", content).await.unwrap();
    
    let new_version = NewFileVersion {
        file_id,
        workspace_id,
        branch: "main".to_string(),
        app_data: serde_json::json!({}),
        hash: hash.clone(),
        author_id: Some(user_id),
    };
    let version = files::create_version(&mut *conn, new_version).await.unwrap();
    files::update_latest_version_id(&mut *conn, file_id, version.id).await.unwrap();

    // Verify file exists in archive
    let archive_path = app.config.storage.base_path.to_string() + "/workspaces/" + &workspace_id.to_string() + "/archive/" + &hash[0..2] + "/" + &hash[2..4] + "/" + &hash;
    assert!(std::path::Path::new(&archive_path).exists(), "Archive blob should exist");

    // 5. Delete the version record (should trigger the cleanup queue)
    sqlx::query!("DELETE FROM file_versions WHERE id = $1", version.id)
        .execute(&mut *conn).await.unwrap();

    // Verify it's in the queue
    let queue_count = sqlx::query!("SELECT count(*) as count FROM file_archive_cleanup_queue WHERE hash = $1", hash)
        .fetch_one(&mut *conn).await.unwrap().count.unwrap();
    assert_eq!(queue_count, 1, "Hash should be in cleanup queue table");

    // 6. Run the cleanup batch
    let processed = process_cleanup_batch(&mut *conn, &storage).await.unwrap();
    assert!(processed >= 1, "Should have processed at least one item");

    // 7. Verify file is GONE from disk
    assert!(!std::path::Path::new(&archive_path).exists(), "Archive blob should be deleted from disk");

    // Verify it's GONE from queue
    let queue_count_after = sqlx::query!("SELECT count(*) as count FROM file_archive_cleanup_queue WHERE hash = $1", hash)
        .fetch_one(&mut *conn).await.unwrap().count.unwrap();
    assert_eq!(queue_count_after, 0, "Hash should be removed from queue");
}

#[tokio::test]
async fn test_archive_cleanup_deduplication_safety() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let mut conn = app.get_connection().await;
    let storage = FileStorageService::new(&app.config.storage.base_path);
    
    // Get current user ID
    let token = register_and_login(&app).await;
    let user_info = app.client.get(&app.url("/api/v1/auth/me"))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    let user_id = Uuid::parse_str(user_info["user"]["id"].as_str().unwrap()).unwrap();

    let workspace_id = Uuid::now_v7();
    sqlx::query!("INSERT INTO workspaces (id, name, owner_id) VALUES ($1, $2, $3)", 
        workspace_id, "Dedupe Test", user_id)
        .execute(&mut *conn).await.unwrap();

    // Create TWO files with SAME content (same hash)
    let content = b"shared content";
    
    // File 1
    let file1 = files::create_file_identity(&mut *conn, NewFile {
        workspace_id, parent_id: None, author_id: user_id,
        file_type: FileType::Document, status: buildscale::models::files::FileStatus::Ready,
        name: "f1.txt".to_string(), slug: "f1.txt".to_string(), path: "/f1.txt".to_string(),
        is_virtual: false, is_remote: false, permission: 600,
    }).await.unwrap();
    let hash = storage.write_file(workspace_id, "/f1.txt", content).await.unwrap();
    let v1 = files::create_version(&mut *conn, NewFileVersion {
        file_id: file1.id, workspace_id, branch: "main".to_string(),
        app_data: serde_json::json!({}), hash: hash.clone(), author_id: Some(user_id),
    }).await.unwrap();

    // File 2
    let file2 = files::create_file_identity(&mut *conn, NewFile {
        workspace_id, parent_id: None, author_id: user_id,
        file_type: FileType::Document, status: buildscale::models::files::FileStatus::Ready,
        name: "f2.txt".to_string(), slug: "f2.txt".to_string(), path: "/f2.txt".to_string(),
        is_virtual: false, is_remote: false, permission: 600,
    }).await.unwrap();
    let v2 = files::create_version(&mut *conn, NewFileVersion {
        file_id: file2.id, workspace_id, branch: "main".to_string(),
        app_data: serde_json::json!({}), hash: hash.clone(), author_id: Some(user_id),
    }).await.unwrap();

    // 1. Delete File 1's version
    sqlx::query!("DELETE FROM file_versions WHERE id = $1", v1.id)
        .execute(&mut *conn).await.unwrap();

    // 2. Run cleanup
    process_cleanup_batch(&mut *conn, &storage).await.unwrap();

    // 3. Verify file STILL EXISTS on disk because File 2 still references it
    let archive_path = app.config.storage.base_path.to_string() + "/workspaces/" + &workspace_id.to_string() + "/archive/" + &hash[0..2] + "/" + &hash[2..4] + "/" + &hash;
    assert!(std::path::Path::new(&archive_path).exists(), "Archive blob should STILL exist due to deduplication");

    // 4. Delete File 2's version
    sqlx::query!("DELETE FROM file_versions WHERE id = $1", v2.id)
        .execute(&mut *conn).await.unwrap();

    // 5. Run cleanup again
    process_cleanup_batch(&mut *conn, &storage).await.unwrap();

    // 6. NOW it should be gone
    assert!(!std::path::Path::new(&archive_path).exists(), "Archive blob should now be deleted");
}
