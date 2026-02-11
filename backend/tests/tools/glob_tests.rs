//! Tests for glob tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_glob_pattern_all_rust_files() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Glob Test").await;

    write_file(&app, &workspace_id, &token, "/main.rs", serde_json::json!("fn main() {}")).await;
    write_file(&app, &workspace_id, &token, "/src/lib.rs", serde_json::json!("pub fn hello() {}")).await;
    write_file(&app, &workspace_id, &token, "/README.md", serde_json::json!("# Test")).await;

    let response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "*.rs"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0]["path"], "/main.rs");
    assert_eq!(matches[1]["path"], "/src/lib.rs");
}

#[tokio::test]
async fn test_glob_pattern_recursive() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Glob Recursive Test").await;

    write_file(&app, &workspace_id, &token, "/src/main.rs", serde_json::json!("main")).await;
    write_file(&app, &workspace_id, &token, "/src/utils/helper.rs", serde_json::json!("helper")).await;
    write_file(&app, &workspace_id, &token, "/tests/test.rs", serde_json::json!("test")).await;
    write_file(&app, &workspace_id, &token, "/README.md", serde_json::json!("readme")).await;

    let response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "**/*.rs"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 3);
}

#[tokio::test]
async fn test_glob_pattern_with_base_path() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Glob Base Path Test").await;

    write_file(&app, &workspace_id, &token, "/src/main.rs", serde_json::json!("main")).await;
    write_file(&app, &workspace_id, &token, "/tests/test.rs", serde_json::json!("test")).await;

    let response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "*.rs",
        "path": "/src"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["path"], "/src/main.rs");
}

#[tokio::test]
async fn test_glob_pattern_no_matches() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Glob No Matches Test").await;

    write_file(&app, &workspace_id, &token, "/README.md", serde_json::json!("readme")).await;

    let response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "*.rs"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 0);
}

#[tokio::test]
async fn test_glob_empty_pattern() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Glob Empty Pattern Test").await;

    let response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": ""
    })).await;

    assert_eq!(response.status(), 400);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_glob_pattern_with_prefix() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Glob Prefix Test").await;

    write_file(&app, &workspace_id, &token, "/test_main.rs", serde_json::json!("test_main")).await;
    write_file(&app, &workspace_id, &token, "/main.rs", serde_json::json!("main")).await;
    write_file(&app, &workspace_id, &token, "/test_helper.rs", serde_json::json!("helper")).await;

    let response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "test_*.rs"
    })).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 2);

    // Check that results are sorted by path
    let paths: Vec<&str> = matches.iter()
        .map(|m| m["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, vec!["/test_helper.rs", "/test_main.rs"]);
}
