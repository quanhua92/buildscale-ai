//! Integration tests for links table and backlink lookups

use serde_json::json;
use std::time::Duration;

mod common;
use common::{TestApp, register_and_login, create_workspace};

/// Test that backlinks are found via links table (not file scanning)
#[tokio::test]
async fn test_backlinks_via_links_table() {
    let app = TestApp::new().await;

    // 1. Create workspace and authenticate
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Test Workspace").await;

    // 2. Create file A that links to file B
    let file_a_response = app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "file-a.md",
            "path": "/file-a.md",
            "file_type": "document",
            "content": "# File A\n\nThis links to [[file-b]]."
        }))
        .send()
        .await
        .expect("Failed to create file A");
    assert!(file_a_response.status().is_success());
    let file_a: serde_json::Value = file_a_response.json().await.unwrap();
    let file_a_id = file_a["file"]["id"].as_str().unwrap();
    println!("File A created: id={}, name={}", file_a_id, file_a["file"]["name"]);

    // 3. Create file B (the target of the link)
    let file_b_response = app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "file-b.md",
            "path": "/file-b.md",
            "file_type": "document",
            "content": "# File B\n\nThis is the target."
        }))
        .send()
        .await
        .expect("Failed to create file B");
    assert!(file_b_response.status().is_success());
    let file_b: serde_json::Value = file_b_response.json().await.unwrap();
    let file_b_id = file_b["file"]["id"].as_str().unwrap();
    println!("File B created: id={}, name={}", file_b_id, file_b["file"]["name"]);

    // 4. Wait for link indexer to process (100ms batch interval + buffer)
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 5. Check the links table directly
    let links: Vec<(String, String)> = sqlx::query_as(
        "SELECT f.name, l.target_name FROM links l JOIN files f ON f.id = l.source_file_id WHERE l.workspace_id = $1"
    )
    .bind(uuid::Uuid::parse_str(&workspace_id).unwrap())
    .fetch_all(&app.pool)
    .await
    .expect("Failed to query links");
    println!("Links in database: {:?}", links);

    // 6. Get network for file B - should show file A in backlinks
    let network_response = app.client
        .get(&app.url(&format!(
            "/api/v1/workspaces/{}/files/{}/network",
            workspace_id, file_b_id
        )))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get file network");

    assert!(network_response.status().is_success());
    let network: serde_json::Value = network_response.json().await.unwrap();

    println!("Network response: {:?}", network);

    // 7. Verify backlinks contains file A
    let backlinks = network["backlinks"].as_array().expect("backlinks should be array");
    assert!(
        backlinks.iter().any(|b| b.as_str() == Some("file-a.md")),
        "file-a.md should be in backlinks, got: {:?}",
        backlinks
    );
}

/// Test that link updates when file content changes
#[tokio::test]
async fn test_links_update_on_edit() {
    let app = TestApp::new().await;

    // 1. Create workspace and authenticate
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Edit Test Workspace").await;

    // 2. Create file A that links to file B
    let file_a_response = app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "source.md",
            "path": "/source.md",
            "file_type": "document",
            "content": "# Source\n\nLinks to [[target]]."
        }))
        .send()
        .await
        .expect("Failed to create file A");
    let file_a: serde_json::Value = file_a_response.json().await.unwrap();
    let _file_a_id = file_a["file"]["id"].as_str().unwrap();

    // 3. Create target file
    let file_b_response = app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "target.md",
            "path": "/target.md",
            "file_type": "document",
            "content": "# Target"
        }))
        .send()
        .await
        .expect("Failed to create file B");
    let file_b: serde_json::Value = file_b_response.json().await.unwrap();
    let file_b_id = file_b["file"]["id"].as_str().unwrap();

    // 4. Wait for initial indexing
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 5. Verify initial backlink
    let network1 = app.client
        .get(&app.url(&format!(
            "/api/v1/workspaces/{}/files/{}/network",
            workspace_id, file_b_id
        )))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get network")
        .json::<serde_json::Value>()
        .await
        .unwrap();

    assert!(
        network1["backlinks"].as_array().unwrap().iter().any(|b| b.as_str() == Some("source.md")),
        "source.md should be in backlinks initially"
    );

    // 6. Edit file A to remove the link
    let _edit_response = app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/tools", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "tool": "edit",
            "args": {
                "path": "/source.md",
                "old_string": "Links to [[target]].",
                "new_string": "No more links."
            }
        }))
        .send()
        .await
        .expect("Failed to edit file");

    // 7. Wait for reindexing
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 8. Verify backlink is removed
    let network2 = app.client
        .get(&app.url(&format!(
            "/api/v1/workspaces/{}/files/{}/network",
            workspace_id, file_b_id
        )))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get network")
        .json::<serde_json::Value>()
        .await
        .unwrap();

    assert!(
        !network2["backlinks"].as_array().unwrap().iter().any(|b| b.as_str() == Some("source.md")),
        "source.md should NOT be in backlinks after edit"
    );
}

/// Test that text_search works (renamed from semantic_search)
#[tokio::test]
async fn test_text_search_works() {
    let app = TestApp::new().await;

    // 1. Create workspace and authenticate
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Search Test Workspace").await;

    // 2. Create a file with searchable content
    let _file_response = app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/files", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "name": "notes.md",
            "path": "/notes.md",
            "file_type": "document",
            "content": "# My Notes\n\nThis contains the word pineapple."
        }))
        .send()
        .await
        .expect("Failed to create file");

    // 3. Search for "pineapple"
    let search_response = app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/search", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "query": "pineapple"
        }))
        .send()
        .await
        .expect("Failed to search");

    assert!(search_response.status().is_success());
    let search_result: serde_json::Value = search_response.json().await.unwrap();

    // 4. Verify results
    assert_eq!(search_result["type"], "text_search");
    let results = search_result["results"].as_array().expect("results should be array");
    assert!(!results.is_empty(), "Should find at least one result");
    assert!(
        results[0]["file"]["name"].as_str() == Some("notes.md"),
        "Should find notes.md"
    );
}
