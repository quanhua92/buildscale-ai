//! Tests for grep tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_grep_basic_search() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Basic Test").await;
    
    // 1. Create files
    write_file(&app, &workspace_id, &token, "/file1.rs", serde_json::json!({"text": "fn main() {\n    println!(\"hello\");\n}"})).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!({"text": "Just some text with main in it."})).await;

    // 2. Perform grep for "main"
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "main"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 2);
    
    // Check first match
    assert_eq!(matches[0]["path"], "/file1.rs");
    assert_eq!(matches[0]["line_number"], 1);
    assert!(matches[0]["line_text"].as_str().unwrap().contains("fn main()"));
    
    // Check second match
    assert_eq!(matches[1]["path"], "/file2.txt");
    assert_eq!(matches[1]["line_number"], 1);
    assert!(matches[1]["line_text"].as_str().unwrap().contains("with main in it"));
}

#[tokio::test]
async fn test_grep_path_filter() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Path Test").await;
    
    write_file(&app, &workspace_id, &token, "/src/main.rs", serde_json::json!({"text": "pattern here"})).await;
    write_file(&app, &workspace_id, &token, "/docs/readme.md", serde_json::json!({"text": "pattern here"})).await;

    // Search only in .rs files
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "pattern",
        "path_pattern": "%.rs"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["path"], "/src/main.rs");
}

#[tokio::test]
async fn test_grep_case_sensitivity() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Case Test").await;
    
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "CASE sensitive\ncase insensitive"})).await;

    // Case-insensitive (default)
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "CASE"
    })).await;
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["matches"].as_array().unwrap().len(), 2);

    // Case-sensitive
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "CASE",
        "case_sensitive": true
    })).await;
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["matches"].as_array().unwrap().len(), 1);
    assert_eq!(body["result"]["matches"][0]["line_number"], 1);
}

#[tokio::test]
async fn test_grep_regex_pattern() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Regex Test").await;
    
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "foo123bar\nfooabcbar"})).await;

    // Match digits only
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "foo[0-9]+bar"
    })).await;
    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["line_text"], "foo123bar");
}

#[tokio::test]
async fn test_grep_raw_string_content() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Raw Test").await;
    
    // Create a file with raw string content (not wrapped in {"text": ...})
    let res = app.client.post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": "raw.txt",
            "file_type": "document",
            "content": "raw string match"
        }))
        .send().await.unwrap();
    assert!(res.status().is_success());

    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "match"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["path"], "/raw.txt");
    assert_eq!(matches[0]["line_text"], "raw string match");
}

#[tokio::test]
async fn test_grep_path_variations() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Path Var Test").await;
    
    write_file(&app, &workspace_id, &token, "/src/main.rs", serde_json::json!({"text": "findme"})).await;
    write_file(&app, &workspace_id, &token, "/config/app.json", serde_json::json!({"text": "findme"})).await;

    let scenarios = vec![
        ("src", 1, "/src/main.rs"),      // Should match directory
        ("/src", 1, "/src/main.rs"),     // Absolute path match
        ("main.rs", 1, "/src/main.rs"),  // Filename match (exact)
        ("src/", 1, "/src/main.rs"),     // Directory with slash
        ("%.rs", 1, "/src/main.rs"),     // SQL wildcard
        ("config", 1, "/config/app.json"),
    ];

    for (pattern, expected_count, expected_path) in scenarios {
        let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
            "pattern": "findme",
            "path_pattern": pattern
        })).await;
        
        let body: serde_json::Value = response.json().await.unwrap();
        let matches = body["result"]["matches"].as_array().unwrap();
        assert_eq!(matches.len(), expected_count, "Failed for path_pattern: {}", pattern);
        if expected_count > 0 {
            assert_eq!(matches[0]["path"], expected_path);
        }
    }
}
