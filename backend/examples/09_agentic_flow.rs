/// Agentic Engine Flow Example: Complex Multi-step Workflow
///
/// This example demonstrates the decoupled Command/Event architecture of the Agentic Engine:
/// 1. **Seed**: Anchor the session identity (Create .chat file).
/// 2. **Environment Setup**: Pre-create files for the agent to manipulate.
/// 3. **Event Pipe**: Establish a long-lived SSE connection for AI feedback.
/// 4. **Complex Task**: Send a multi-step instruction (List -> Rename -> Append -> Delete).
///
/// **Usage:**
/// ```bash
/// cargo run --example 09_agentic_flow
/// ```

use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

fn get_base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000/api/v1".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(60))
        .build()?;

    let api_base_url = get_base_url();

    println!("🚀 Agentic Engine Flow Example: Complex Task Verification");
    println!("====================================================\n");

    // 1. Authentication & Setup
    println!("1️⃣  Authenticating...");
    let email = format!("agent_test_{}@example.com", Uuid::now_v7());
    let password = "SecurePass123!";
    
    // Register
    client.post(&format!("{}/auth/register", api_base_url))
        .json(&json!({
            "email": email,
            "password": password,
            "confirm_password": password,
            "full_name": "Agent Tester"
        }))
        .send()
        .await?;

    // Login
    let login_res = client.post(&format!("{}/auth/login", api_base_url))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    let token = login_res["access_token"]
        .as_str()
        .expect("access_token not found in login response");
    println!("✓ Authenticated as: {}\n", email);

    // 2. Create Workspace
    println!("2️⃣  Creating Workspace...");
    let ws_res = client.post(&format!("{}/workspaces", api_base_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "name": "Agentic Sandbox", "slug": format!("ws-{}", Uuid::now_v7()) }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    let workspace_id = ws_res["workspace"]["id"]
        .as_str()
        .expect("workspace id not found in response");
    println!("✓ Workspace Created: {}\n", workspace_id);

    // 3. Setup Environment Files
    println!("3️⃣  Seeding test environment files...");
    // Create temp_file.txt
    client.post(&format!("{}/workspaces/{}/files", api_base_url, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "name": "temp_file.txt", "file_type": "document", "content": "This file will be renamed." }))
        .send().await?;
    
    // Create delete_me.md
    client.post(&format!("{}/workspaces/{}/files", api_base_url, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "name": "delete_me.md", "file_type": "document", "content": "This file will be deleted." }))
        .send().await?;

    // Create append_target.txt
    client.post(&format!("{}/workspaces/{}/files", api_base_url, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "name": "append_target.txt", "file_type": "document", "content": "Original log entry." }))
        .send().await?;
    println!("✓ Environment ready.\n");

    // 4. Phase 1: The Seed (Anchor Identity)
    println!("4️⃣  Phase 1: Seeding the Chat session...");
    let chat_res = client.post(&format!("{}/workspaces/{}/chats", api_base_url, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "goal": "I want to perform workspace maintenance: list, rename, append, and delete.",
            "agents": []
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    let chat_id = chat_res["chat_id"]
        .as_str()
        .expect("chat_id not found in response");
    println!("✓ Session Anchored. Chat ID: {}\n", chat_id);

    // 5. Phase 2: The Event Pipe (Standard Out)
    println!("5️⃣  Phase 2: Connecting to Event Pipe (SSE)...");
    let res = client.get(&format!("{}/workspaces/{}/chats/{}/events", api_base_url, workspace_id, chat_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let mut event_stream = res.bytes_stream();
    println!("✓ Event Pipe Connected. Listening for AI actions...\n");

    // 6. Phase 3: The Command Bus (Follow-up)
    println!("6️⃣  Phase 3: Sending complex task command...");
    let prompt = r#"
Please perform the following maintenance tasks in order:
1. List the files in the root directory to see what's there.
2. Rename 'temp_file.txt' to 'buildscale_renamed.txt' using the 'mv' tool.
3. Read 'append_target.txt', then use the 'write' tool to append the line ' - Verified by Agent' to its content.
4. Remove 'delete_me.md' using the 'rm' tool.
"#;

    client.post(&format!("{}/workspaces/{}/chats/{}", api_base_url, workspace_id, chat_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "content": prompt }))
        .send()
        .await?;

    println!("✓ Command Dispatched. Processing stream...\n");

    // 7. Process SSE Stream
    println!("--- [ AI EXECUTION LOG ] ---");
    
    while let Some(item) = event_stream.next().await {
        let chunk = item?;
        let text = String::from_utf8_lossy(&chunk);
        
        for line in text.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..];
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                    let event_type = event["type"].as_str().expect("Event type missing");
                    match event_type {
                        "thought" => println!("🤔 THOUGHT: {}", event["data"]["text"].as_str().unwrap_or("...")),
                        "call" => println!("🛠️  CALL: {} with args {}", event["data"]["tool"], event["data"]["args"]),
                        "observation" => println!("👁️  OBSERVATION: Tool output received."),
                        "chunk" => print!("{}", event["data"]["text"].as_str().unwrap_or("")),
                        "ping" => print!("."),
                        "done" => {
                            println!("\n\n✅ DONE: {}", event["data"]["message"]);
                            return Ok(());
                        }
                        "error" => {
                            println!("\n❌ ERROR: {}", event["data"]["message"]);
                            return Ok(());
                        }
                        _ => {}
                    }
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }
            }
        }
    }

    Ok(())
}
