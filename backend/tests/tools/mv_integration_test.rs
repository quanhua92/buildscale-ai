//! Test to demonstrate the mv tool bug
//!
//! This test shows that:
//! - `mv` tool updates the database but does NOT move files on disk
//! - `find` and `glob` tools show inconsistent results after mv
//!
//! BUG SCENARIO:
//! 1. Create file at /test_file.txt (DB + disk)
//! 2. Move to /moved/test_file.txt using mv tool
//! 3. mv updates DB to new path
//! 4. mv does NOT move file on disk (BUG!)
//! 5. Result: File exists at old path on disk, but DB says it's at new path
//!
//! This test should FAIL until the mv tool is fixed.

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, write_file};

#[tokio::test]
async fn test_mv_move_file_and_verify_with_find_and_glob() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "MV Tool Bug Test").await;

    // Step 1: Create a file via API (will be in DB + on disk)
    write_file(&app, &workspace_id, &token, "/test_file.txt", serde_json::json!("Original content")).await;

    // Create destination directory
    execute_tool(&app, &workspace_id, &token, "mkdir", serde_json::json!({
        "path": "/moved"
    })).await;

    println!("\n=== STEP 1: File created at /test_file.txt, directory /moved/ created ===");

    // Verify file exists at original location
    let read_response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": "/test_file.txt"
    })).await;
    assert_eq!(read_response.status(), 200);
    let read_body: serde_json::Value = read_response.json().await.unwrap();
    assert!(read_body["success"].as_bool().unwrap());
    assert_eq!(read_body["result"]["content"], "Original content");

    // Step 2: Move file using mv tool (directory will be auto-created)
    println!("\n=== STEP 2: Moving /test_file.txt to /moved/ ===");

    let mv_response = execute_tool(&app, &workspace_id, &token, "mv", serde_json::json!({
        "source": "/test_file.txt",
        "destination": "/moved/"
    })).await;

    assert_eq!(mv_response.status(), 200);
    let mv_body: serde_json::Value = mv_response.json().await.unwrap();
    assert!(mv_body["success"].as_bool().unwrap());

    println!("mv result: {:?}", mv_body);

    // Step 3: Verify file is accessible at NEW path (via API/read tool)
    println!("\n=== STEP 3: Verify file accessible at new path ===");

    // When moving to a directory, the to_path in response shows the directory,
    // but the actual file path should be /moved/test_file.txt
    let new_path = "/moved/test_file.txt";
    println!("File moved to: {}", new_path);

    let read_new_response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({
        "path": new_path
    })).await;

    assert_eq!(read_new_response.status(), 200);
    let read_new_body: serde_json::Value = read_new_response.json().await.unwrap();
    assert!(read_new_body["success"].as_bool().unwrap());
    assert_eq!(read_new_body["result"]["content"], "Original content");

    println!("✓ File accessible via read tool at new path: {}", new_path);

    // Step 4: Verify find tool finds file at NEW path
    println!("\n=== STEP 4: Check find tool ===");

    let find_response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "test_file.txt"
    })).await;

    assert_eq!(find_response.status(), 200);
    let find_body: serde_json::Value = find_response.json().await.unwrap();
    assert!(find_body["success"].as_bool().unwrap());

    let find_matches: Vec<&str> = find_body["result"]["matches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["path"].as_str().unwrap())
        .collect();

    println!("find found {} files:", find_matches.len());
    for path in &find_matches {
        println!("  - {}", path);
    }

    // Step 5: Verify glob tool finds file at NEW path
    println!("\n=== STEP 5: Check glob tool ===");

    let glob_response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "**/test_file.txt"
    })).await;

    assert_eq!(glob_response.status(), 200);
    let glob_body: serde_json::Value = glob_response.json().await.unwrap();
    assert!(glob_body["success"].as_bool().unwrap());

    let glob_matches: Vec<&str> = glob_body["result"]["matches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["path"].as_str().unwrap())
        .collect();

    println!("glob found {} files:", glob_matches.len());
    for path in &glob_matches {
        println!("  - {}", path);
    }

    // === THE BUG ASSERTIONS ===
    println!("\n=== BUG VERIFICATION ===");

    // BUG: Both find and glob should find the file at the NEW path
    assert!(find_matches.contains(&new_path),
        "find should find file at NEW path {}", new_path);
    assert!(glob_matches.contains(&new_path),
        "glob should find file at NEW path {}", new_path);

    println!("✓ Both tools found file at new path: {}", new_path);

    // BUG: Neither tool should find the file at the OLD path
    assert!(!find_matches.contains(&"/test_file.txt"),
        "find should NOT find file at OLD path /test_file.txt (it was moved)");
    assert!(!glob_matches.contains(&"/test_file.txt"),
        "glob should NOT find file at OLD path /test_file.txt (it was moved)");

    println!("✓ Neither tool found file at old path");

    // BUG: Both tools should agree on the file location
    assert_eq!(find_matches.len(), glob_matches.len(),
        "find and glob should find the same number of files");
    assert_eq!(find_matches, glob_matches,
        "find and glob should find files at the same paths");

    println!("✓ find and glob agree on file locations");

    println!("\n✅ TEST PASSED - mv tool correctly moves files on disk");
}
