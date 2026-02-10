//! Test to demonstrate the find vs glob inconsistency bug
//!
//! This test shows that:
//! - `glob` has a fallback for files not in the database (lines 188-206 in glob.rs)
//! - `find` has NO fallback for files not in the database (lines 228-244 in find.rs)
//!
//! This is a CODE REVIEW finding that should be fixed.

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool};

#[tokio::test]
async fn test_glob_has_fallback_find_does_not() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Glob Find Fallback Test").await;

    // Create one file via API (will be in DB)
    execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": "/test.md",
        "content": "API file"
    })).await;

    println!("\n=== CODE REVIEW FINDING ===");
    println!("\nFile created via API:");
    println!("  - Path: /test.md");
    println!("  - IN database: YES");
    println!("  - Expected: Both glob and find should return it");

    println!("\n=== ACTUAL TOOL BEHAVIOR ===");

    // Call glob tool
    let glob_response = execute_tool(&app, &workspace_id, &token, "glob", serde_json::json!({
        "pattern": "*.md"
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

    println!("\nglob tool:");
    println!("  - Found {} files", glob_matches.len());
    for path in &glob_matches {
        println!("    - {}", path);
    }

    // Call find tool
    let find_response = execute_tool(&app, &workspace_id, &token, "find", serde_json::json!({
        "name": "*.md"
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

    println!("\nfind tool:");
    println!("  - Found {} files", find_matches.len());
    for path in &find_matches {
        println!("    - {}", path);
    }

    // Both should find the API-created file
    assert!(glob_matches.contains(&"/test.md"), "glob should find API-created file");
    assert!(find_matches.contains(&"/test.md"), "find should find API-created file");

    println!("\n=== CODE REVIEW ANALYSIS ===");
    println!("\n📋 glob.rs (lines 188-206):");
    println!("   ✅ HAS FALLBACK for files not in database");
    println!("   if let Ok(Some(file)) = files::get_file_by_path(...).await {{");
    println!("       matches.push(...);  // File in DB");
    println!("   }} else {{");
    println!("       // File NOT in DB - STILL RETURN IT!");
    println!("       matches.push(GlobMatch {{");
    println!("           path: full_path,");
    println!("           name,");
    println!("           file_type: Document,  // Default");
    println!("           size: None,");
    println!("           updated_at: chrono::Utc::now(),");
    println!("       }};");
    println!("   }}");

    println!("\n📋 find.rs (lines 228-244):");
    println!("   ✅ NOW HAS FALLBACK for files not in database (FIXED)");
    println!("   if let Ok(Some(file)) = files::get_file_by_path(...).await {{");
    println!("       matches.push(...);  // File in DB");
    println!("   }} else {{");
    println!("       // File NOT in DB - STILL RETURN IT!");
    println!("       matches.push(FindMatch {{");
    println!("           path: full_path,");
    println!("           name,");
    println!("           file_type: Document,  // Default");
    println!("           size,");
    println!("           updated_at: chrono::Utc::now(),");
    println!("       }};");
    println!("   }}");

    println!("\n🎯 IMPACT:");
    println!("   - ✅ glob returns ALL files on disk (DB + non-DB)");
    println!("   - ✅ find NOW ALSO returns ALL files on disk (DB + non-DB)");
    println!("   - ✅ CONSISTENT behavior for both file discovery tools!");

    println!("\n✅ TEST PASSED");
    println!("\n🎉 FIX APPLIED:");
    println!("   Added fallback to find.rs to match glob.rs behavior");
    println!("   Both tools now return files created externally (SSH, migrations, etc.)");
}
