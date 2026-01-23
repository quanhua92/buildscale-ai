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
    assert_eq!(ls_body["result"]["entries"].as_array().unwrap().len(), 3);
    
    delete_file(&app, &workspace_id, &token, "/folder/sub1/a.txt").await;
    delete_file(&app, &workspace_id, &token, "/folder/sub1/b.txt").await;
    delete_file(&app, &workspace_id, &token, "/folder/sub2/c.txt").await;
    
    delete_file(&app, &workspace_id, &token, "/folder/sub1").await;
    delete_file(&app, &workspace_id, &token, "/folder/sub2").await;
    delete_file(&app, &workspace_id, &token, "/folder").await;
}
