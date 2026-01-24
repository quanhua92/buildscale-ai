/// Agentic Engine Flow Example
///
/// This example demonstrates the decoupled Command/Event architecture of the Agentic Engine:
/// 1. **Seed**: Anchor the session identity (Create .chat file).
/// 2. **Event Pipe**: Establish a long-lived SSE connection for AI feedback.
/// 3. **Command Bus**: Send user interactions as non-blocking POST requests.
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

    println!("🚀 Agentic Engine Flow Example");
    println!("==============================\n");

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
    
    let token = login_res["access_token"].as_str().unwrap();
    println!("✓ Authenticated as: {}\n", email);

    // 2. Get Workspace
    println!("2️⃣  Creating Workspace...");
    let ws_res = client.post(&format!("{}/workspaces", api_base_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "name": "Agentic Sandbox", "slug": format!("ws-{}", Uuid::now_v7()) }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    let workspace_id = ws_res["workspace"]["id"].as_str().unwrap();
    println!("✓ Workspace Created: {}\n", workspace_id);

    // 3. Phase 1: The Seed (Anchor Identity)
    println!("3️⃣  Phase 1: Seeding the Chat session...");
    let chat_res = client.post(&format!("{}/workspaces/{}/chats", api_base_url, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "goal": "Hello! I want to start an engineering session in this workspace.",
            "agents": []
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    let chat_id = chat_res["chat_id"].as_str().unwrap();
    println!("✓ Session Anchored. Chat ID: {}\n", chat_id);

    // 4. Phase 2: The Event Pipe (Standard Out)
    println!("4️⃣  Phase 2: Connecting to Event Pipe (SSE)...");
    let res = client.get(&format!("{}/workspaces/{}/chats/{}/events", api_base_url, workspace_id, chat_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let mut event_stream = res.bytes_stream();

    println!("✓ Event Pipe Connected. Listening for AI thoughts...\n");

    // 5. Phase 3: The Command Bus (Follow-up)
    println!("5️⃣  Phase 3: Sending interaction command...");
    client.post(&format!("{}/workspaces/{}/chats/{}", api_base_url, workspace_id, chat_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "content": "Please create a file named 'buildscale_status.txt' in the root directory with the content 'The Agentic Engine is operational and verified.'" }))
        .send()
        .await?;

    println!("✓ Command Dispatched. Waiting for AI response...\n");

    // 6. Process SSE Stream
    println!("--- [ AI OUTPUT STREAM ] ---");
    
    while let Some(item) = event_stream.next().await {
        let chunk = item?;
        let text = String::from_utf8_lossy(&chunk);
        
        // Simple SSE line parser
        for line in text.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..];
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                    match event["type"].as_str().unwrap() {
                        "thought" => println!("🤔 THOUGHT: {}", event["data"]["text"].as_str().unwrap_or("...")),
                        "call" => println!("🛠️  CALL: {} with args {}", event["data"]["tool"], event["data"]["args"]),
                        "observation" => println!("👁️  OBSERVATION: {}", event["data"]["output"]),
                        "chunk" => print!("{}", event["data"]["text"].as_str().unwrap_or("")),
                        "ping" => {
                            // Heartbeat - normally silent or small indicator
                            print!(".");
                        }
                        "done" => {
                            println!("\n\n✅ DONE: {}", event["data"]["message"]);
                            return Ok(());
                        }
                        "error" => {
                            println!("\n❌ ERROR: {}", event["data"]["message"]);
                            return Ok(());
                        }
                        _ => println!("\n📢 EVENT: {:?}", event),
                    }
                    // Flush output
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }
            }
        }
    }

    Ok(())
}
