//! Integration tests for tools

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file, read_file, delete_file};

#[tokio::test]
async fn test_full_workflow() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Integration Workflow Test").await;
    
    let content = serde_json::json!({"text": "workflow test content"});
    
    write_file(&app, &workspace_id, &token, "/workflow.md", content.clone()).await;
    read_file(&app, &workspace_id, &token, "/workflow.md").await;
    delete_file(&app, &workspace_id, &token, "/workflow.md").await;
    
    let read_response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/workflow.md"
    })).await;
    assert_eq!(read_response.status(), 404);
}

#[tokio::test]
async fn test_multiple_files_workflow() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Multiple Files Test").await;
    
    write_file(&app, &workspace_id, &token, "/file1.txt", serde_json::json!({"text": "content1"})).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!({"text": "content2"})).await;
    write_file(&app, &workspace_id, &token, "/file3.txt", serde_json::json!({"text": "content3"})).await;
    
    let ls_response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({})).await;
    assert_eq!(ls_response.status(), 200);
    let ls_body: serde_json::Value = ls_response.json().await.unwrap();
    assert_eq!(ls_body["result"]["entries"].as_array().unwrap().len(), 3);
    
    read_file(&app, &workspace_id, &token, "/file1.txt").await;
    read_file(&app, &workspace_id, &token, "/file2.txt").await;
    read_file(&app, &workspace_id, &token, "/file3.txt").await;
    
    delete_file(&app, &workspace_id, &token, "/file1.txt").await;
    delete_file(&app, &workspace_id, &token, "/file2.txt").await;
    delete_file(&app, &workspace_id, &token, "/file3.txt").await;
    
    let final_ls = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({})).await;
    let final_body: serde_json::Value = final_ls.json().await.unwrap();
    assert_eq!(final_body["result"]["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_folder_structure_workflow() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Folder Structure Test").await;
    
    write_file(&app, &workspace_id, &token, "/folder/sub1/a.txt", serde_json::json!({"text": "a"})).await;
    write_file(&app, &workspace_id, &token, "/folder/sub1/b.txt", serde_json::json!({"text": "b"})).await;
    write_file(&app, &workspace_id, &token, "/folder/sub2/c.txt", serde_json::json!({"text": "c"})).await;
    
    let ls_response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/folder",
        "recursive": true
    })).await;
    assert_eq!(ls_response.status(), 200);
    let ls_body: serde_json::Value = ls_response.json().await.unwrap();
    assert_eq!(ls_body["result"]["entries"].as_array().unwrap().len(), 5);
    
    delete_file(&app, &workspace_id, &token, "/folder/sub1/a.txt").await;
    delete_file(&app, &workspace_id, &token, "/folder/sub1/b.txt").await;
    delete_file(&app, &workspace_id, &token, "/folder/sub2/c.txt").await;
    
    delete_file(&app, &workspace_id, &token, "/folder/sub1").await;
    delete_file(&app, &workspace_id, &token, "/folder/sub2").await;
    delete_file(&app, &workspace_id, &token, "/folder").await;
}

// New integration tests for tool combinations

#[tokio::test]
async fn test_glob_to_find_to_file_info_workflow() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Glob-Find-FileInfo Workflow").await;

    // Setup: Create multiple test files
    write_file(&app, &workspace_id, &token, "/config/app.json", serde_json::json!("{}")).await;
    write_file(&app, &workspace_id, &token, "/config/database.json", serde_json::json!("{}")).await;
    write_file(&app, &workspace_id, &token, "/src/main.rs", serde_json::json!("fn main() {}")).await;
    write_file(&app, &workspace_id, &token, "/README.md", serde_json::json!("# Project")).await;

    // Step 1: Use glob to find all JSON files
    let glob_response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "**/*.json"
    })).await;

    assert_eq!(glob_response.status(), 200);
    let glob_body: serde_json::Value = glob_response.json().await.unwrap();
    assert!(glob_body["success"].as_bool().unwrap());

    let glob_matches = glob_body["result"]["matches"].as_array().unwrap();
    assert!(glob_matches.len() >= 2, "Should find at least 2 JSON files");

    // Step 2: Use find to filter by file type and size
    let find_response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.json"
    })).await;

    assert_eq!(find_response.status(), 200);
    let find_body: serde_json::Value = find_response.json().await.unwrap();
    assert!(find_body["success"].as_bool().unwrap());

    // Step 3: Use file_info to get metadata for specific files
    for file_match in glob_matches.iter() {
        let path = file_match["path"].as_str().unwrap();

        let info_response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
            "path": path
        })).await;

        assert_eq!(info_response.status(), 200);
        let info_body: serde_json::Value = info_response.json().await.unwrap();
        assert!(info_body["success"].as_bool().unwrap());
        assert_eq!(info_body["result"]["path"], path);
        assert!(info_body["result"]["hash"].as_str().is_some());
    }
}

#[tokio::test]
async fn test_glob_to_read_multiple_files_workflow() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Glob-ReadMultiple Workflow").await;

    // Setup: Create multiple configuration files
    write_file(&app, &workspace_id, &token, "/config/app.yml", serde_json::json!("port: 3000")).await;
    write_file(&app, &workspace_id, &token, "/config/db.yml", serde_json::json!("host: localhost")).await;
    write_file(&app, &workspace_id, &token, "/config/cache.yml", serde_json::json!("ttl: 3600")).await;

    // Step 1: Use glob to discover all YAML files
    let glob_response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "**/*.yml"
    })).await;

    assert_eq!(glob_response.status(), 200);
    let glob_body: serde_json::Value = glob_response.json().await.unwrap();
    assert!(glob_body["success"].as_bool().unwrap());

    let glob_matches = glob_body["result"]["matches"].as_array().unwrap();
    assert!(glob_matches.len() >= 3, "Should find at least 3 YAML files");

    // Step 2: Extract paths and use read_multiple_files to batch read them
    let paths: Vec<String> = glob_matches.iter()
        .map(|m| m["path"].as_str().unwrap().to_string())
        .collect();

    let read_response = execute_tool(&app, &workspace_id, &token, "read_multiple_files", serde_json::json!({
        "paths": paths
    })).await;

    assert_eq!(read_response.status(), 200);
    let read_body: serde_json::Value = read_response.json().await.unwrap();
    assert!(read_body["success"].as_bool().unwrap());

    let files = read_body["result"]["files"].as_array().unwrap();
    assert!(files.len() >= 3, "Should read at least 3 files");

    // Verify all files were read successfully
    for file_result in files.iter() {
        assert!(file_result["success"].as_bool().unwrap());
        assert!(file_result["content"].as_str().is_some());
        assert!(file_result["hash"].as_str().is_some());
    }
}

#[tokio::test]
async fn test_find_to_cat_workflow() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Find-Cat Workflow").await;

    // Setup: Create log files with different sizes
    write_file(&app, &workspace_id, &token, "/logs/app.log", serde_json::json!("Error: Something went wrong\nInfo: Starting up")).await;
    write_file(&app, &workspace_id, &token, "/logs/system.log", serde_json::json!("Warning: Low memory\nInfo: System check")).await;
    write_file(&app, &workspace_id, &token, "/logs/debug.log", serde_json::json!("Debug: Variable x = 10\nDebug: Variable y = 20")).await;

    // Step 1: Use find to locate all log files
    let find_response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.log"
    })).await;

    assert_eq!(find_response.status(), 200);
    let find_body: serde_json::Value = find_response.json().await.unwrap();
    assert!(find_body["success"].as_bool().unwrap());

    let find_matches = find_body["result"]["matches"].as_array().unwrap();
    assert!(find_matches.len() >= 3, "Should find at least 3 log files");

    // Step 2: Extract paths and use cat to display with line numbers
    let paths: Vec<String> = find_matches.iter()
        .map(|m| m["path"].as_str().unwrap().to_string())
        .collect();

    let cat_response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": paths,
        "number_lines": true
    })).await;

    assert_eq!(cat_response.status(), 200);
    let cat_body: serde_json::Value = cat_response.json().await.unwrap();
    assert!(cat_body["success"].as_bool().unwrap());

    // Verify content includes line numbers
    let content = cat_body["result"]["content"].as_str().unwrap();
    // Line numbers are formatted as 6-digit numbers followed by tab: "     1\t"
    assert!(content.contains("     1\t"), "Content should have line numbers");
}

#[tokio::test]
async fn test_error_handling_in_tool_chain() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Error Handling Workflow").await;

    // Setup: Create some files
    write_file(&app, &workspace_id, &token, "/valid.txt", serde_json::json!("content")).await;

    // Step 1: Use glob to find files (including non-existent ones)
    let glob_response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "*.txt"
    })).await;

    assert_eq!(glob_response.status(), 200);
    let glob_body: serde_json::Value = glob_response.json().await.unwrap();
    assert!(glob_body["success"].as_bool().unwrap());

    // Step 2: Try to read multiple files where some don't exist
    let read_response = execute_tool(&app, &workspace_id, &token, "read_multiple_files", serde_json::json!({
        "paths": ["/valid.txt", "/nonexistent.txt", "/also_missing.txt"]
    })).await;

    assert_eq!(read_response.status(), 200);
    let read_body: serde_json::Value = read_response.json().await.unwrap();
    assert!(read_body["success"].as_bool().unwrap());

    let files = read_body["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 3);

    // Verify partial success handling
    let valid_file = &files[0];
    assert!(valid_file["success"].as_bool().unwrap());
    assert_eq!(valid_file["path"], "/valid.txt");

    let missing_file = &files[1];
    assert!(!missing_file["success"].as_bool().unwrap());
    assert!(missing_file["error"].as_str().is_some());
}

#[tokio::test]
async fn test_size_based_workflow() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Size-Based Workflow").await;

    // Setup: Create files of different sizes
    write_file(&app, &workspace_id, &token, "/tiny.txt", serde_json::json!("a")).await; // 1 byte
    write_file(&app, &workspace_id, &token, "/small.txt", serde_json::json!("Hello")).await; // 5 bytes
    write_file(&app, &workspace_id, &token, "/medium.txt", serde_json::json!("Hello World! This is medium.")).await; // 27 bytes
    write_file(&app, &workspace_id, &token, "/large.txt", serde_json::json!("A".repeat(100))).await; // 100 bytes

    // Step 1: Find files with size >= 10 bytes
    let find_response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "min_size": 10
    })).await;

    assert_eq!(find_response.status(), 200);
    let find_body: serde_json::Value = find_response.json().await.unwrap();
    assert!(find_body["success"].as_bool().unwrap());

    let matches = find_body["result"]["matches"].as_array().unwrap();
    assert!(matches.len() >= 2, "Should find at least 2 files with size >= 10");

    // Step 2: Use file_info to verify sizes
    for file_match in matches.iter() {
        let path = file_match["path"].as_str().unwrap();
        let reported_size = file_match["size"].as_u64().unwrap();

        let info_response = execute_tool(&app, &workspace_id, &token, "file_info", serde_json::json!({
            "path": path
        })).await;

        assert_eq!(info_response.status(), 200);
        let info_body: serde_json::Value = info_response.json().await.unwrap();
        assert!(info_body["success"].as_bool().unwrap());

        let actual_size = info_body["result"]["size"].as_u64().unwrap();
        assert_eq!(reported_size, actual_size, "Find and file_info should report same size");
        assert!(actual_size >= 10, "All files should have size >= 10");
    }
}

