/// Tools API Example
///
/// This example demonstrates how to use the Tools API endpoint
/// to execute filesystem operations within a workspace.
///
/// **Environment Variables:**
/// - `API_BASE_URL`: API base URL (default: http://localhost:3000/api/v1)
///
/// **Usage:**
/// ```bash
/// # Use default URL (http://localhost:3000/api/v1)
/// cargo run --example 08_tools_api
///
/// # Use custom URL
/// API_BASE_URL=http://localhost:3001/api/v1 cargo run --example 08_tools_api
/// ```
///
/// **Prerequisites:**
/// 1. Start the server: `cargo run` (from backend directory)
/// 2. Ensure the database is running and migrations are applied
///
/// **What this example demonstrates:**
/// - User registration and login
/// - Workspace creation
/// - ls: List directory contents (non-recursive and recursive)
/// - write: Create new files with auto-folder creation
/// - read: Read file contents
/// - write: Update existing files (versioning)
/// - rm: Delete files (soft delete)
/// - Error handling (file not found, invalid tool, etc.)
///
/// **Note:** This is a client-side example that makes HTTP requests to the API.
/// The Tools API provides a unified endpoint for all file operations.

use reqwest::Client;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

fn get_base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000/api/v1".to_string())
}

/// Generate a unique email for testing to avoid conflicts
fn generate_test_email() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("tools_api_{}@example.com", timestamp)
}

/// Generate a unique workspace name for testing
fn generate_test_workspace_name() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("Tools API Test Workspace {}", timestamp)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize HTTP client
    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    let api_base_url = get_base_url();

    println!("🔧 Tools API Example");
    println!("===================\n");
    println!("Making requests to: {}", api_base_url);
    println!();

    // ========================================================
    // STEP 1: Check server health
    // ========================================================
    println!("1️⃣  Checking server health...");
    match check_server_health(&client, &api_base_url).await {
        Ok(()) => println!("✓ Server is running and healthy\n"),
        Err(e) => {
            println!("✗ Server health check failed: {}", e);
            println!("\n💡 Make sure to start the server with: cargo run");
            return Err(e.into());
        }
    }

    // ========================================================
    // STEP 2: Register and login user
    // ========================================================
    println!("2️⃣  Setting up user account...");
    let email = generate_test_email();
    let password = "ToolsAPISecure2026!";

    match register_user(&client, &api_base_url, &email, password).await {
        Ok(_) => {
            println!("✓ User registered: {}", email);
            println!();
        }
        Err(e) => {
            println!("✗ Registration failed: {}", e);
            return Err(e.into());
        }
    }

    let (access_token, _refresh_token, user_id) = match login_user(&client, &api_base_url, &email, password).await {
        Ok(tokens) => {
            println!("✓ Login successful!");
            println!("  User ID: {}", tokens.0);
            println!();
            tokens
        }
        Err(e) => {
            println!("✗ Login failed: {}", e);
            return Err(e.into());
        }
    };

    // ========================================================
    // STEP 3: Create workspace
    // ========================================================
    println!("3️⃣  Creating workspace...");
    let workspace_id = match create_workspace(&client, &api_base_url, &access_token, &generate_test_workspace_name()).await {
        Ok(id) => {
            println!("✓ Workspace created: {}", id);
            println!();
            id
        }
        Err(e) => {
            println!("✗ Workspace creation failed: {}", e);
            return Err(e.into());
        }
    };

    // ========================================================
    // STEP 4: ls tool - List directory (non-recursive)
    // ========================================================
    println!("4️⃣  Testing 'ls' tool - List root directory (non-recursive)...");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "ls",
        json!({ "path": "/", "recursive": false }),
    ).await {
        Ok(result) => {
            println!("✓ ls command successful!");
            if let Some(entries) = result["result"]["entries"].as_array() {
                println!("  Found {} entries in root directory", entries.len());
                if entries.is_empty() {
                    println!("  (Directory is empty)");
                } else {
                    for entry in entries {
                        println!("  - {} ({})", entry["name"], entry["file_type"]);
                    }
                }
            }
            println!();
        }
        Err(e) => {
            println!("✗ ls command failed: {}", e);
            println!();
        }
    }

    // ========================================================
    // STEP 5: write tool - Create files
    // ========================================================
    println!("5️⃣  Testing 'write' tool - Create new files...");

    // Create a file at root
    println!("  a) Creating file at root: /README.md");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "write",
        json!({
            "path": "/README.md",
            "content": { "text": "# Tools API Demo\n\nThis is a demo file created via the Tools API." }
        }),
    ).await {
        Ok(result) => {
            println!("  ✓ File created!");
            println!("    File ID: {}", result["result"]["file_id"]);
            println!("    Version ID: {}", result["result"]["version_id"]);
        }
        Err(e) => {
            println!("  ✗ File creation failed: {}", e);
        }
    }

    // Create nested file (auto-folder creation)
    println!("  b) Creating nested file (auto-folder creation): /docs/notes.md");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "write",
        json!({
            "path": "/docs/notes.md",
            "content": { "text": "# Notes\n\nAuto-created nested folder structure!" }
        }),
    ).await {
        Ok(result) => {
            println!("  ✓ Nested file created!");
            println!("    File ID: {}", result["result"]["file_id"]);
        }
        Err(e) => {
            println!("  ✗ Nested file creation failed: {}", e);
        }
    }

    // Create another file for versioning demo
    println!("  c) Creating file for versioning demo: /version-test.txt");
    let version_test_result = execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "write",
        json!({
            "path": "/version-test.txt",
            "content": { "text": "Initial version" }
        }),
    ).await;

    let initial_version_id = match &version_test_result {
        Ok(result) => {
            println!("  ✓ File created!");
            let vid = result["result"]["version_id"].as_str().unwrap_or("");
            println!("    Initial Version ID: {}", vid);
            vid.to_string()
        }
        Err(e) => {
            println!("  ✗ File creation failed: {}", e);
            String::new()
        }
    };

    println!();

    // ========================================================
    // STEP 6: ls tool - List with recursive option
    // ========================================================
    println!("6️⃣  Testing 'ls' tool - List with recursive option...");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "ls",
        json!({ "path": "/", "recursive": true }),
    ).await {
        Ok(result) => {
            println!("✓ Recursive ls successful!");
            if let Some(entries) = result["result"]["entries"].as_array() {
                println!("  Found {} entries (recursive)", entries.len());
                for entry in entries {
                    println!("  - {} ({})", entry["path"], entry["file_type"]);
                }
            }
            println!();
        }
        Err(e) => {
            println!("✗ Recursive ls failed: {}", e);
            println!();
        }
    }

    // ========================================================
    // STEP 7: read tool - Read file contents
    // ========================================================
    println!("7️⃣  Testing 'read' tool - Read file contents...");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "read",
        json!({ "path": "/docs/notes.md" }),
    ).await {
        Ok(result) => {
            println!("✓ Read successful!");
            if let Some(content) = result["result"]["content"].as_str() {
                println!("  Content preview: {}...", &content[..content.len().min(50)]);
            } else if let Some(content_obj) = result["result"]["content"].as_object() {
                if let Some(text) = content_obj.get("text").and_then(|v| v.as_str()) {
                    println!("  Content preview: {}...", &text[..text.len().min(50)]);
                }
            }
            println!();
        }
        Err(e) => {
            println!("✗ Read failed: {}", e);
            println!();
        }
    }

    // ========================================================
    // STEP 8: read tool - Error handling (file not found)
    // ========================================================
    println!("8️⃣  Testing 'read' tool - Error handling (file not found)...");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "read",
        json!({ "path": "/nonexistent/file.txt" }),
    ).await {
        Ok(_) => {
            println!("✗ Read should have failed for non-existent file");
        }
        Err(e) => {
            println!("✓ Correctly failed with error: {}", e);
            println!();
        }
    }

    // ========================================================
    // STEP 9: write tool - Update existing file (versioning)
    // ========================================================
    println!("9️⃣  Testing 'write' tool - Update existing file (versioning)...");
    let update_result = execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "write",
        json!({
            "path": "/version-test.txt",
            "content": { "text": "Updated version - second version" }
        }),
    ).await;

    let _new_version_id = match &update_result {
        Ok(result) => {
            println!("✓ File updated!");
            let vid = result["result"]["version_id"].as_str().unwrap_or("");
            println!("  New Version ID: {}", vid);
            if !initial_version_id.is_empty() && initial_version_id != vid {
                println!("  ✓ Version ID changed (versioning confirmed)");
            }
            vid.to_string()
        }
        Err(e) => {
            println!("✗ File update failed: {}", e);
            String::new()
        }
    };

    println!();

    // ========================================================
    // STEP 10: rm tool - Delete file
    // ========================================================
    println!("🔟 Testing 'rm' tool - Delete file...");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "rm",
        json!({ "path": "/version-test.txt" }),
    ).await {
        Ok(result) => {
            println!("✓ File deleted (soft delete)!");
            println!("  File ID: {}", result["result"]["file_id"]);
            println!();
        }
        Err(e) => {
            println!("✗ File deletion failed: {}", e);
            println!();
        }
    }

    // ========================================================
    // STEP 11: rm tool - Error handling (already deleted)
    // ========================================================
    println!("1️⃣1️⃣  Testing 'rm' tool - Error handling (already deleted)...");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "rm",
        json!({ "path": "/version-test.txt" }),
    ).await {
        Ok(_) => {
            println!("✗ Delete should have failed for already deleted file");
        }
        Err(e) => {
            println!("✓ Correctly failed with error: {}", e);
            println!();
        }
    }

    // ========================================================
    // STEP 12: Error scenarios
    // ========================================================
    println!("1️⃣2️⃣  Testing error scenarios...");

    // Invalid tool name
    println!("  a) Testing invalid tool name...");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "invalid_tool",
        json!({}),
    ).await {
        Ok(_) => {
            println!("  ✗ Should have failed for invalid tool");
        }
        Err(e) => {
            println!("  ✓ Correctly failed: {}", e);
        }
    }

    // Missing required arguments
    println!("  b) Testing missing required arguments...");
    match execute_tool(
        &client,
        &api_base_url,
        &workspace_id,
        &access_token,
        "read",
        json!({}), // missing 'path'
    ).await {
        Ok(_) => {
            println!("  ✗ Should have failed for missing arguments");
        }
        Err(e) => {
            println!("  ✓ Correctly failed: {}", e);
        }
    }

    println!();

    // ========================================================
    // SUMMARY
    // ========================================================
    println!("===================");
    println!("✅ Tools API example completed successfully!");
    println!();
    println!("📝 Key Takeaways:");
    println!("  • Unified endpoint: POST /api/v1/workspaces/:id/tools");
    println!("  • All tools use same request format: {{ tool: ..., args: {{...}} }}");
    println!("  • Response format: {{ success: ..., result: {{...}}, error: ... }}");
    println!("  • JWT authentication required (Authorization: Bearer <token>)");
    println!("  • Workspace isolation: Tools operate within workspace context");
    println!("  • Auto-folder creation: write tool creates nested paths");
    println!("  • Versioning: write creates new version for existing files");
    println!("  • Soft delete: rm tool preserves data (sets deleted_at)");
    println!("  • Error handling: Clear error messages with codes");
    println!();

    // ========================================================
    // CLEANUP (Optional)
    // ========================================================
    println!("💡 Note: Test data remains in workspace for inspection.");
    println!("  Workspace ID: {}", workspace_id);
    println!("  User ID: {}", user_id);
    println!("  You can manually delete this workspace and user if needed.");
    println!();

    Ok(())
}

/// Check if the server is running and healthy
async fn check_server_health(
    client: &Client,
    base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .get(&format!("{}/health", base_url))
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Server returned status: {}", response.status()).into())
    }
}

/// Register a new user
async fn register_user(
    client: &Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/auth/register", base_url);
    let request_body = json!({
        "email": email,
        "password": password,
        "confirm_password": password,
        "full_name": "Tools API Test User"
    });

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        return Err(format!("Registration failed ({}): {}", status, body).into());
    }

    Ok(())
}

/// Login user and return (access_token, refresh_token, user_id)
async fn login_user(
    client: &Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let url = format!("{}/auth/login", base_url);
    let request_body = json!({
        "email": email,
        "password": password
    });

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        return Err(format!("Login failed ({}): {}", status, body).into());
    }

    let json: serde_json::Value = response.json().await?;
    let access_token = json["access_token"].as_str().unwrap_or("").to_string();
    let refresh_token = json["refresh_token"].as_str().unwrap_or("").to_string();
    let user_id = json["user"]["id"].as_str().unwrap_or("").to_string();

    Ok((access_token, refresh_token, user_id))
}

/// Create a workspace and return its ID
async fn create_workspace(
    client: &Client,
    base_url: &str,
    access_token: &str,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("{}/workspaces", base_url);
    let request_body = json!({ "name": name });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        return Err(format!("Workspace creation failed ({}): {}", status, body).into());
    }

    let json: serde_json::Value = response.json().await?;
    let workspace_id = json["workspace"]["id"].as_str().unwrap_or("").to_string();
    Ok(workspace_id)
}

/// Execute a tool and return the result
async fn execute_tool(
    client: &Client,
    base_url: &str,
    workspace_id: &str,
    access_token: &str,
    tool: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let url = format!("{}/workspaces/{}/tools", base_url, workspace_id);
    let request_body = json!({
        "tool": tool,
        "args": args
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        return Err(format!("Tool execution failed ({}): {}", status, body).into());
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;

    // Check if tool execution itself failed
    if json["success"].as_bool() != Some(true) {
        let error_msg = json["error"].as_str().unwrap_or("Unknown error");
        return Err(format!("Tool error: {}", error_msg).into());
    }

    Ok(json)
}
