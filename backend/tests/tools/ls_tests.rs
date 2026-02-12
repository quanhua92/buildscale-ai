//! Tests for ls tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_ls_root_directory() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Test").await;
    
    write_file(&app, &workspace_id, &token, "/file1.md", serde_json::json!({"text": "content1"})).await;
    write_file(&app, &workspace_id, &token, "/file2.md", serde_json::json!({"text": "content2"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({})).await;
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["entries"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_ls_nested_directory() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Nested Test").await;
    
    write_file(&app, &workspace_id, &token, "/folder/nested.md", serde_json::json!({"text": "nested"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/folder"
    })).await;
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(body["result"]["entries"][0]["name"], "nested.md");
}

#[tokio::test]
async fn test_ls_recursive() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Recursive Test").await;
    
    write_file(&app, &workspace_id, &token, "/folder/subfolder/nested.md", serde_json::json!({"text": "nested"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/folder",
        "recursive": true
    })).await;
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["entries"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_ls_empty_directory() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Empty Test").await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({})).await;
    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    assert_eq!(body["result"]["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_ls_nonexistent_path() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Nonexistent Test").await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/nonexistent"
    })).await;
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_ls_file_as_directory() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS File Validation Test").await;
    
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "content"})).await;
    
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/test.txt"
    })).await;
    
    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["fields"]["path"].as_str().unwrap().contains("Path is not a directory"));
}

#[tokio::test]
async fn test_ls_hybrid_discovery_external_file() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Hybrid Test").await;

    // Create one file via the API (in database)
    write_file(&app, &workspace_id, &token, "/api_file.md", serde_json::json!({"text": "api content"})).await;

    // Create another file directly on disk (not in database)
    // This simulates files created via SSH, migration scripts, etc.
    let workspace_storage_path = std::path::PathBuf::from("./storage/workspaces").join(&workspace_id).join("latest");
    tokio::fs::create_dir_all(&workspace_storage_path).await.unwrap();
    let external_file_path = workspace_storage_path.join("external_file.md");
    tokio::fs::write(&external_file_path, "external content").await.unwrap();

    // List directory - should see BOTH files
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({})).await;
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let entries = body["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2, "Should see both API file and external file");

    // Find API file (has id)
    let api_entry = entries.iter().find(|e| e["name"] == "api_file.md").unwrap();
    assert!(api_entry["id"].is_string(), "API file should have a database ID");

    // Find external file (no id)
    let external_entry = entries.iter().find(|e| e["name"] == "external_file.md").unwrap();
    assert!(external_entry["id"].is_null() || external_entry["id"].as_str().map(|s| s.is_empty()).unwrap_or(true),
            "External file should not have a database ID");
}

#[tokio::test]
async fn test_ls_hybrid_discovery_recursive_external() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "LS Hybrid Recursive Test").await;

    // Create a file in subdirectory via API
    write_file(&app, &workspace_id, &token, "/folder/api_file.md", serde_json::json!({"text": "api content"})).await;

    // Create external file in the same subdirectory directly on disk
    let workspace_storage_path = std::path::PathBuf::from("./storage/workspaces").join(&workspace_id).join("latest");
    let external_dir = workspace_storage_path.join("folder");
    tokio::fs::create_dir_all(&external_dir).await.unwrap();
    let external_file_path = external_dir.join("external_file.md");
    tokio::fs::write(&external_file_path, "external content").await.unwrap();

    // List root recursively - should see both files
    let response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "recursive": true
    })).await;
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let entries = body["result"]["entries"].as_array().unwrap();
    // Should see: folder (database), folder/api_file.md (database), folder/external_file.md (filesystem)
    assert_eq!(entries.len(), 3, "Should see folder and both files in recursive listing");

    // Verify we have the folder
    let folder_entry = entries.iter().find(|e| e["path"] == "/folder" && e["file_type"] == "folder").unwrap();
    assert_eq!(folder_entry["name"], "folder");

    // Verify we have the API file (has ID)
    let api_file_entry = entries.iter().find(|e| e["path"] == "/folder/api_file.md").unwrap();
    assert!(api_file_entry["id"].is_string(), "API file should have a database ID");

    // Verify we have the external file (no ID)
    let external_file_entry = entries.iter().find(|e| e["path"] == "/folder/external_file.md").unwrap();
    assert!(external_file_entry["id"].is_null() || external_file_entry["id"].as_str().map(|s| s.is_empty()).unwrap_or(true),
            "External file should not have a database ID");
}
