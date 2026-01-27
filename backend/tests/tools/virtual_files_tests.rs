//! Tests for virtual files (Chat, etc.) behavior with tools
//! Covers: Visibility (ls), Protection (write/edit), and Write-Through (read/grep)

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};
use crate::tools::common::{execute_tool, read_file};

#[tokio::test]
async fn test_virtual_file_lifecycle() {
    let app = TestApp::new_with_options(TestAppOptions::api()).await;
    let token = register_and_login(&app).await;
    let workspace_id = create_workspace(&app, &token, "Virtual Files Test").await;

    // 1. Create a Chat via API
    let chat_response = app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/chats", workspace_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "goal": "Test Chat Goal"
        }))
        .send()
        .await
        .unwrap();
    
    assert_eq!(chat_response.status(), 201);
    let chat_body: serde_json::Value = chat_response.json().await.unwrap();
    let chat_id = chat_body["chat_id"].as_str().unwrap();
    
    // We don't know the exact slug/path yet, so let's find it with ls
    // Chats are in /chats folder
    let ls_response = execute_tool(&app, &workspace_id, &token, "ls", serde_json::json!({
        "path": "/chats"
    })).await;
    
    assert_eq!(ls_response.status(), 200);
    let ls_body: serde_json::Value = ls_response.json().await.unwrap();
    let entries = ls_body["result"]["entries"].as_array().unwrap();
    assert!(!entries.is_empty());
    
    let chat_entry = &entries[0];
    let chat_path = chat_entry["path"].as_str().unwrap();
    
    // 2. Verify Visibility (is_virtual = true)
    assert!(chat_entry["is_virtual"].as_bool().unwrap(), "Chat file should be virtual");
    assert_eq!(chat_entry["file_type"].as_str().unwrap(), "chat");

    // 3. Verify Protection (Write Blocked)
    let write_response = execute_tool(&app, &workspace_id, &token, "write", serde_json::json!({
        "path": chat_path,
        "content": {"some": "junk"}
    })).await;
    
    assert_eq!(write_response.status(), 400); // Bad Request (Validation Error)
    let write_err = write_response.json::<serde_json::Value>().await.unwrap();
    // Check validation error structure
    assert_eq!(write_err["code"].as_str().unwrap(), "VALIDATION_ERROR");
    // The specific message is in the fields
    let field_error = if let Some(errs) = write_err["fields"]["path"].as_array() {
        errs[0].as_str().unwrap()
    } else {
        write_err["fields"]["path"].as_str().unwrap()
    };
    assert!(field_error.contains("Cannot write to a virtual file"));

    // 4. Verify Protection (Edit Blocked)
    let edit_response = execute_tool(&app, &workspace_id, &token, "edit", serde_json::json!({
        "path": chat_path,
        "old_string": "Goal",
        "new_string": "Hacked"
    })).await;
    
    assert_eq!(edit_response.status(), 400);
    let edit_err = edit_response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(edit_err["code"].as_str().unwrap(), "VALIDATION_ERROR");
    let edit_field_error = if let Some(errs) = edit_err["fields"]["path"].as_array() {
        errs[0].as_str().unwrap()
    } else {
        edit_err["fields"]["path"].as_str().unwrap()
    };
    assert!(edit_field_error.contains("Cannot edit a virtual file"));

    // 5. Verify Write-Through (Read)
    // The initial creation should have snapshotted the goal message
    let read_content = read_file(&app, &workspace_id, &token, chat_path).await;
    // Content should be the ChatSession JSON
    assert!(read_content["messages"].as_array().unwrap().len() >= 1);
    let first_msg = &read_content["messages"][0];
    assert_eq!(first_msg["content"].as_str().unwrap(), "Test Chat Goal");

    // 6. Verify Write-Through (Update via API -> Read)
    // Post a new message
    let msg_content = "UniqueSearchToken123";
    let post_response = app.client
        .post(&app.url(&format!("/api/v1/workspaces/{}/chats/{}", workspace_id, chat_id)))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "content": msg_content
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(post_response.status(), 202);

    // Read again - should contain new message
    let read_content_2 = read_file(&app, &workspace_id, &token, chat_path).await;
    let messages_2 = read_content_2["messages"].as_array().unwrap();
    assert!(messages_2.iter().any(|m| m["content"].as_str() == Some(msg_content)));

    // 7. Verify Write-Through (Grep)
    let grep_response = execute_tool(&app, &workspace_id, &token, "grep", serde_json::json!({
        "pattern": "UniqueSearchToken123"
    })).await;
    
    assert_eq!(grep_response.status(), 200);
    let grep_body: serde_json::Value = grep_response.json().await.unwrap();
    let matches = grep_body["result"]["matches"].as_array().unwrap();
    
    assert!(!matches.is_empty(), "Grep should find the chat message");
    assert_eq!(matches[0]["path"].as_str().unwrap(), chat_path);
}
