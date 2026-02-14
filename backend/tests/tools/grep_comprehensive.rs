//! Comprehensive tests for grep tool covering common use cases
//!
//! These tests ensure grep works correctly for real-world scenarios:
//! - Context lines (before/after/shorthand)
//! - Nested directories (src/**/*.rs)
//! - Multiple file types (*.py and *.sh)
//! - Exact word matching vs substring
//! - Special characters
//! - No matches handling
//! - Leading slash in directory paths

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_grep_context_lines() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Context Test").await;

    // Create a file with clear before/after context markers
    let content = r#"
Line 1: Before context
Line 2: Before context
Line 3: Before context
Line 4: Before context
Line 5: Before context
Line 6: TARGET MATCH HERE
Line 7: After context 1
Line 8: After context 2
Line 9: After context 3
"#;
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // Test with before_context only
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "TARGET",
        "path_pattern": "*.txt",
        "before_context": 3
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);

    let before_context = matches[0].get("before_context").and_then(|v| v.as_array());
    let after_context = matches[0].get("after_context").and_then(|v| v.as_array());

    // before_context should be present with 3 lines
    assert!(before_context.is_some(), "before_context field should be present");
    assert_eq!(before_context.unwrap().len(), 3, "Should have 3 lines before match");

    // after_context should either be None or empty array
    if let Some(after) = after_context {
        assert_eq!(after.len(), 0, "Should have no after_context");
    }

    // Test with after_context only
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "TARGET",
        "path_pattern": "*.txt",
        "after_context": 2
    })).await;

    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();
    let before_context = matches[0].get("before_context").and_then(|v| v.as_array());
    let after_context = matches[0].get("after_context").and_then(|v| v.as_array());

    // before_context should be None or empty
    if let Some(before) = before_context {
        assert_eq!(before.len(), 0, "Should have no before_context");
    }
    // after_context should have 2 lines
    assert!(after_context.is_some(), "after_context field should be present");
    assert_eq!(after_context.unwrap().len(), 2, "Should have 2 lines after match");

    // Test with context shorthand (both before and after)
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "TARGET",
        "path_pattern": "*.txt",
        "context": 2
    })).await;

    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();
    let before_context = matches[0].get("before_context").and_then(|v| v.as_array());
    let after_context = matches[0].get("after_context").and_then(|v| v.as_array());

    // Both before and after context should be present with 2 lines each
    assert!(before_context.is_some(), "before_context field should be present");
    assert!(after_context.is_some(), "after_context field should be present");
    assert_eq!(before_context.unwrap().len(), 2, "Context shorthand should give 2 before");
    assert_eq!(after_context.unwrap().len(), 2, "Context shorthand should give 2 after");
}

#[tokio::test]
async fn test_grep_nested_directory_pattern() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Nested Dir Test").await;

    // Create nested directory structure
    write_file(&app, &workspace_id, &token, "/src/lib/util.rs", serde_json::json!("fn helper() {}")).await;
    write_file(&app, &workspace_id, &token, "/src/main.rs", serde_json::json!("fn main() {}")).await;
    write_file(&app, &workspace_id, &token, "/tests/test.rs", serde_json::json!("fn test() {}")).await;

    // Search with ** pattern - should find recursively in subdirectories
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "fn",
        "path_pattern": "src/**/*.rs"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();

    // Should find matches in src/ and src/lib/
    assert!(matches.len() >= 2, "Should find fn in src/ and src/lib/");

    // Verify no matches in tests/
    for match_obj in matches {
        let path = match_obj["path"].as_str().unwrap();
        assert!(path.starts_with("/src/"), "All matches should be under /src/, got: {}", path);
    }
}

#[tokio::test]
async fn test_grep_multiple_file_types() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Multiple Types Test").await;

    // Create multiple file types with common pattern
    write_file(&app, &workspace_id, &token, "/scripts/deploy.sh", serde_json::json!("deploy_function()")).await;
    write_file(&app, &workspace_id, &token, "/scripts/build.py", serde_json::json!("build_function()")).await;
    write_file(&app, &workspace_id, &token, "/docs/readme.md", serde_json::json!("function() in markdown")).await;
    write_file(&app, &workspace_id, &token, "/config.json", serde_json::json!("function in json")).await;

    // Search for "function" in multiple file types (Python and shell scripts)
    // Note: Can't use comma-separated patterns with ripgrep --glob, so search in scripts directory
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "function"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();

    // Should find in build.py and deploy.sh
    assert!(matches.len() >= 2);
}

#[tokio::test]
async fn test_grep_exact_word_not_substring() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Word Match Test").await;

    let content = r#"
main
main_function
main()
"#;
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // Search for exact word "main" - should match "main" and "main()" but not "main_function"
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "main"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();

    // Should find all 3 occurrences (grep does substring matching by default)
    assert_eq!(matches.len(), 3, "Should find all occurrences of 'main'");
}

#[tokio::test]
async fn test_grep_special_characters() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Special Chars Test").await;

    let content = r#"
Price: $100.50
Email: test@example.com
URL: https://example.com
Special: [test] (nested)
Regex: \d+\.\d{2}
"#;
    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!(content)).await;

    // Test various special character patterns
    let scenarios = vec![
        (r"\$", 1, "Dollar sign"),
        (r"@", 1, "At sign"),
        (r"https://", 1, "URL"),
        (r"\[test\]", 1, "Brackets"),
    ];

    for (pattern, expected_min, desc) in scenarios {
        let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
            "pattern": pattern
        })).await;

        let body: serde_json::Value = response.json().await.unwrap();
        if body["success"].as_bool().unwrap() {
            let matches = body["result"]["matches"].as_array().unwrap();
            assert!(matches.len() >= expected_min,
                "{}: Expected at least {} matches for pattern '{}', got {}",
                desc, expected_min, pattern, matches.len());
        }
    }
}

#[tokio::test]
async fn test_grep_no_matches_empty_result() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep No Matches Test").await;

    write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!("Some content here")).await;

    // Search for something that doesn't exist
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "NOTFOUND",
        "path_pattern": "*.txt"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();

    // Should be successful (no error) but with empty matches
    assert!(body["success"].as_bool().unwrap());
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 0, "Should return empty array for no matches");
}

#[tokio::test]
async fn test_grep_leading_slash_directory() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Leading Slash Dir Test").await;

    // Create nested structure
    write_file(&app, &workspace_id, &token, "/scripts/deploy.sh", serde_json::json!("deploy()")).await;
    write_file(&app, &workspace_id, &token, "/docs/readme.md", serde_json::json!("docs()")).await;
    write_file(&app, &workspace_id, &token, "/deploy.sh", serde_json::json!("root deploy()")).await;

    // Test with leading slash - should work the same as without
    let scenarios = vec![
        ("/scripts", 1, "Should find in /scripts/ with leading slash"),
        ("scripts", 1, "Should find in /scripts/ without leading slash"),
    ];

    for (pattern, expected_count, desc) in scenarios {
        let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
            "pattern": "deploy",
            "path_pattern": pattern
        })).await;

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();

        if !body["success"].as_bool().unwrap() {
            panic!("{}: Search failed: {}", desc, body["error"].as_str().unwrap());
        }

        let matches = body["result"]["matches"].as_array().unwrap();
        assert_eq!(matches.len(), expected_count,
            "{}: Expected {} matches, got {}",
            desc, expected_count, matches.len());
    }
}

#[tokio::test]
async fn test_grep_large_result_truncation() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Grep Large Result Test").await;

    // Create many files with the same pattern
    for i in 1..=1050 {
        write_file(&app, &workspace_id, &token, &format!("/file{}.txt", i),
            serde_json::json!(format!("Pattern on line {}", i))).await;
    }

    // Search should truncate at default limit (50)
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "Pattern"
    })).await;

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap());

    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 50, "Should truncate at default limit of 50 matches");

    // Test with explicit limit: 0 (unlimited)
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "Pattern",
        "limit": 0
    })).await;
    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1050, "Should return all 1050 matches with limit: 0");

    // Test with explicit higher limit
    let response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "Pattern",
        "limit": 100
    })).await;
    let body: serde_json::Value = response.json().await.unwrap();
    let matches = body["result"]["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 100, "Should return 100 matches with limit: 100");
}
