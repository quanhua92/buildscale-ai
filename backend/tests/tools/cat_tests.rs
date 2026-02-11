//! Tests for cat tool

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_cat_multiple_files() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Multiple Files Test").await;

    write_file(&app, &workspace_id, &token, "/file1.txt", serde_json::json!("line 1\nline 2")).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!("line 3\nline 4")).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file1.txt", "/file2.txt"]
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let content = body["result"]["content"].as_str().unwrap();
    assert!(content.contains("line 1"));
    assert!(content.contains("line 3"));

    let files = body["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
}

#[tokio::test]
async fn test_cat_with_headers() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat With Headers Test").await;

    write_file(&app, &workspace_id, &token, "/file1.txt", serde_json::json!("content")).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!("content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file1.txt", "/file2.txt"],
        "show_headers": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let content = body["result"]["content"].as_str().unwrap();
    assert!(content.contains("==> /file1.txt <=="));
    assert!(content.contains("==> /file2.txt <=="));
}

#[tokio::test]
async fn test_cat_with_line_numbers() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Line Numbers Test").await;

    let content = "line 1\nline 2\nline 3";
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "number_lines": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let content = body["result"]["content"].as_str().unwrap();
    assert!(content.contains("1\tline 1"));
    assert!(content.contains("2\tline 2"));
    assert!(content.contains("3\tline 3"));
}

#[tokio::test]
async fn test_cat_empty_paths() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Empty Paths Test").await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": []
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_cat_too_many_paths() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Too Many Paths Test").await;

    // Create 21 paths (over the limit of 20)
    let paths: Vec<String> = (0..21).map(|i| format!("/file{}.txt", i)).collect();

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": paths
    })).await;

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_cat_partial_failure() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Partial Failure Test").await;

    write_file(&app, &workspace_id, &token, "/existing.txt", serde_json::json!("content")).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/existing.txt", "/nonexistent.txt"]
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let files = body["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);

    // First file should succeed
    assert_eq!(files[0]["path"], "/existing.txt");
    assert_eq!(files[0]["line_count"], 1);

    // Second file should fail
    assert_eq!(files[1]["path"], "/nonexistent.txt");
    let content = files[1]["content"].as_str().unwrap();
    assert!(content.contains("Error reading"));
}

#[tokio::test]
async fn test_cat_show_ends() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Show Ends Test").await;

    let content = "line 1   \nline 2\nline 3   "; // Trailing spaces
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "show_ends": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let result_content = body["result"]["content"].as_str().unwrap();

    assert!(result_content.contains("line 1   $"));
    assert!(result_content.contains("line 2$"));
    assert!(result_content.contains("line 3   $"));
}

#[tokio::test]
async fn test_cat_show_tabs() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Show Tabs Test").await;

    let content = "line\t1\n\tindented\nmixed\t \tspaces";
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "show_tabs": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let result_content = body["result"]["content"].as_str().unwrap();

    assert!(result_content.contains("line^I1"));
    assert!(result_content.contains("^Iindented"));
    assert!(result_content.contains("mixed^I ^Ispaces"));
}

#[tokio::test]
async fn test_cat_squeeze_blank() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Squeeze Blank Test").await;

    let content = "line 1\n\n\n\nline 2\n\n\nline 3"; // Multiple blank lines
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "squeeze_blank": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let result_content = body["result"]["content"].as_str().unwrap();

    // Should squeeze repeated blank lines
    assert!(result_content.matches("\n\n").count() <= 2);
}

#[tokio::test]
async fn test_cat_combined_special_chars() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Combined Test").await;

    let content = "line\t1   \n\n\nline\t2   ";
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "show_ends": true,
        "show_tabs": true,
        "squeeze_blank": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let result_content = body["result"]["content"].as_str().unwrap();

    assert!(result_content.contains("line^I1   $"));
    assert!(result_content.contains("line^I2   $"));
}

#[tokio::test]
async fn test_cat_with_offset() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Offset Test").await;

    let content = "line 1\nline 2\nline 3\nline 4\nline 5";
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "offset": 2,
        "limit": 2
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let result_content = body["result"]["content"].as_str().unwrap();

    // Should only contain lines 3-4 (0-based offset: lines 2-3)
    assert!(result_content.contains("line 3"));
    assert!(result_content.contains("line 4"));
    assert!(!result_content.contains("line 1"));
    assert!(!result_content.contains("line 5"));

    // Check metadata
    let files = body["result"]["files"].as_array().unwrap();
    assert_eq!(files[0]["offset"], 2);
    assert_eq!(files[0]["limit"], 2);
    assert_eq!(files[0]["total_lines"], 5);
}

#[tokio::test]
async fn test_cat_with_negative_offset() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Negative Offset Test").await;

    let content = "line 1\nline 2\nline 3\nline 4\nline 5";
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "offset": -2,
        "limit": 10
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let result_content = body["result"]["content"].as_str().unwrap();

    // Should contain last 2 lines
    assert!(result_content.contains("line 4"));
    assert!(result_content.contains("line 5"));
    assert!(!result_content.contains("line 1"));
    assert!(!result_content.contains("line 2"));
}

#[tokio::test]
async fn test_cat_offset_with_smart_line_numbering() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Smart Line Numbers Test").await;

    let content = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8";
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "offset": 5,
        "limit": 3,
        "number_lines": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let result_content = body["result"]["content"].as_str().unwrap();

    // Line numbers should start at 6 (offset + 1 for 1-based indexing)
    assert!(result_content.contains("     6\tline 6"));
    assert!(result_content.contains("     7\tline 7"));
    assert!(result_content.contains("     8\tline 8"));
    assert!(!result_content.contains("     1\tline 1"));
}

#[tokio::test]
async fn test_cat_offset_with_special_chars() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Offset Special Chars Test").await;

    let content = "line\t1\nline\t2\nline\t3\nline\t4\nline\t5";
    write_file(&app, &workspace_id, &token, "/file.txt", serde_json::json!(content)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file.txt"],
        "offset": 2,
        "limit": 2,
        "show_tabs": true,
        "number_lines": true
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let result_content = body["result"]["content"].as_str().unwrap();

    // Should show lines 3-4 with tabs as ^I and line numbers starting at 3
    assert!(result_content.contains("     3\tline^I3"));
    assert!(result_content.contains("     4\tline^I4"));
}

#[tokio::test]
async fn test_cat_offset_multiple_files() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Cat Offset Multiple Files Test").await;

    let content1 = "line 1\nline 2\nline 3\nline 4\nline 5";
    let content2 = "line 1\nline 2\nline 3";
    write_file(&app, &workspace_id, &token, "/file1.txt", serde_json::json!(content1)).await;
    write_file(&app, &workspace_id, &token, "/file2.txt", serde_json::json!(content2)).await;

    let response = execute_tool(&app, &workspace_id, &token, "cat", serde_json::json!({
        "paths": ["/file1.txt", "/file2.txt"],
        "offset": 1,
        "limit": 2
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let files = body["result"]["files"].as_array().unwrap();

    // Both files should have same offset/limit applied
    assert_eq!(files[0]["line_count"], 2);
    assert_eq!(files[0]["offset"], 1);
    assert_eq!(files[0]["total_lines"], 5);

    assert_eq!(files[1]["line_count"], 2);
    assert_eq!(files[1]["offset"], 1);
    assert_eq!(files[1]["total_lines"], 3);
}
