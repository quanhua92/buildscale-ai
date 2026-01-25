/// Agentic Engine Flow Example: Complex Multi-step Multi-turn Workflow
///
/// This example demonstrates the full suite of Agentic Engine tools:
/// - Basic: `ls`, `read`, `write`, `rm`, `mv`, `touch`, `mkdir`
/// - Advanced: `edit` (precision), `edit-many` (global), `grep` (SQL-backed search)
///
/// Workflow:
/// 1. **Seed**: Anchor session identity.
/// 2. **Environment Setup**: Seed a mock project structure.
/// 3. **Turn 1 (Exploration)**: List files and search for patterns.
/// 4. **Turn 2 (Precision)**: Read a config and perform a hash-protected edit.
/// 5. **Turn 3 (Global)**: Refactor multiple instances across a file.
/// 6. **Turn 4 (Cleanup)**: Move, delete, and verify.
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
use bytes::Bytes;

fn get_base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000/api/v1".to_string())
}

/// Helper to wait for the agent to finish its current turn by processing the SSE stream.
async fn wait_for_agent_completion(
    mut event_stream: impl futures::Stream<Item = reqwest::Result<Bytes>> + Unpin,
) -> Result<(), Box<dyn std::error::Error>> {
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
                        "observation" => println!("👁️  OBSERVATION: {}", event["data"]["output"].as_str().unwrap_or("Tool execution completed")),
                        "chunk" => print!("{}", event["data"]["text"].as_str().unwrap_or("")),
                        "ping" => print!("."),
                        "done" => {
                            println!("\n\n✅ DONE: {}", event["data"]["message"]);
                            println!("---------------------------\n");
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(120)) // Increased timeout for multi-turn
        .build()?;

    let api_base_url = get_base_url();

    println!("🚀 Agentic Engine Flow Example: Full Tool Suite & Multi-Turn Verification");
    println!("========================================================================\n");

    // 1. Authentication & Setup
    println!("1️⃣  Authenticating...");
    let email = format!("agent_test_{}@example.com", Uuid::now_v7());
    let password = "SecurePass123!";
    
    client.post(&format!("{}/auth/register", api_base_url))
        .json(&json!({
            "email": email, "password": password, "confirm_password": password, "full_name": "Agent Tester"
        }))
        .send().await?;

    let login_res = client.post(&format!("{}/auth/login", api_base_url))
        .json(&json!({ "email": email, "password": password }))
        .send().await?.json::<serde_json::Value>().await?;
    
    let token = login_res["access_token"].as_str().expect("access_token missing");
    println!("✓ Authenticated as: {}\n", email);

    // 2. Create Workspace
    println!("2️⃣  Creating Workspace...");
    let ws_res = client.post(&format!("{}/workspaces", api_base_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "name": "Tool Mastery Sandbox", "slug": format!("ws-{}", Uuid::now_v7()) }))
        .send().await?.json::<serde_json::Value>().await?;
    
    let workspace_id = ws_res["workspace"]["id"].as_str().expect("workspace id missing");
    println!("✓ Workspace Created: {}\n", workspace_id);

    // 3. Setup Environment Files
    println!("3️⃣  Seeding test project structure...");
    
    // Create src/main.rs with multiple DEBUG tags
    let main_rs_content = "fn main() {\n    DEBUG: Initializing engine...\n    DEBUG: Loading modules...\n    println!(\"Hello BuildScale!\");\n}";
    println!("📄 Seeding 'src/main.rs'...");
    let res = client.post(&format!("{}/workspaces/{}/files", api_base_url, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ 
            "name": "main.rs",
            "path": "src/main.rs", 
            "file_type": "document", 
            "content": main_rs_content
        }))
        .send().await?;
    if !res.status().is_success() {
        panic!("Failed to seed src/main.rs: {}", res.text().await?);
    }
    println!("✓ 'src/main.rs' seeded with content:\n---\n{}\n---\n", main_rs_content);
    
    // Create config/app.json for precision editing
    let app_json_content = "{\n  \"env\": \"development\",\n  \"version\": \"1.0.0\"\n}";
    println!("📄 Seeding 'config/app.json'...");
    let res = client.post(&format!("{}/workspaces/{}/files", api_base_url, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ 
            "name": "app.json",
            "path": "config/app.json", 
            "file_type": "document", 
            "content": app_json_content
        }))
        .send().await?;
    if !res.status().is_success() {
        panic!("Failed to seed config/app.json: {}", res.text().await?);
    }
    println!("✓ 'config/app.json' seeded with content:\n---\n{}\n---\n", app_json_content);

    // Create a temporary file for deletion demo
    let temp_notes_content = "This is a temporary note.";
    println!("📄 Seeding 'temp_notes.txt'...");
    let res = client.post(&format!("{}/workspaces/{}/files", api_base_url, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ 
            "name": "temp_notes.txt", 
            "file_type": "document", 
            "content": temp_notes_content
        }))
        .send().await?;
    if !res.status().is_success() {
        panic!("Failed to seed temp_notes.txt: {}", res.text().await?);
    }
    println!("✓ 'temp_notes.txt' seeded with content:\n---\n{}\n---\n", temp_notes_content);

    println!("✓ Project structure seeded.\n");

    // 4. Phase 1: The Seed (Anchor Identity)
    println!("4️⃣  Phase 1: Seeding the Chat session...");
    let chat_res = client.post(&format!("{}/workspaces/{}/chats", api_base_url, workspace_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "goal": "I want to demonstrate all tools in the engine. IMPORTANT: Always prefer 'edit' or 'edit-many' for partial file changes. Use 'write' ONLY for creating entirely new files. Use 'mkdir' for directories. Never use 'json' or 'text' as file types; use 'document' if unsure.",
            "agents": []
        }))
        .send().await?
        .json::<serde_json::Value>().await?;
    
    let chat_id = chat_res["chat_id"].as_str().expect("chat_id missing");
    println!("✓ Session Anchored. Chat ID: {}\n", chat_id);

    // 5. Phase 2: Open Event Pipe
    println!("5️⃣  Phase 2: Connecting to Event Pipe (SSE)...");
    let res = client.get(&format!("{}/workspaces/{}/chats/{}/events", api_base_url, workspace_id, chat_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await?;

    let event_stream = res.bytes_stream();
    let mut event_stream = Box::pin(event_stream);
    println!("✓ Event Pipe Connected.\n");

    // --- TURN 1: Exploration ---
    println!("👉 TURN 1: Exploration (ls, grep)");
    let prompt = "I need to find technical debt. First, list all files recursively. Then, YOU MUST use the 'grep' tool specifically to search for the pattern 'DEBUG' in the '/src' directory. DO NOT read files to find the pattern; use grep.";
    println!("💬 PROMPT: {}", prompt);
    client.post(&format!("{}/workspaces/{}/chats/{}", api_base_url, workspace_id, chat_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "content": prompt }))
        .send().await?;
    wait_for_agent_completion(&mut event_stream).await?;

    // --- TURN 2: Precision Edit ---
    println!("👉 TURN 2: Precision Edit (read, edit)");
    let prompt = "I want to change the environment to production. First, 'read' 'config/app.json' to get its content and hash. Then, YOU MUST use the 'edit' tool (NOT 'write') to replace 'development' with 'production'. You MUST pass the 'last_read_hash' argument for safety.";
    println!("💬 PROMPT: {}", prompt);
    client.post(&format!("{}/workspaces/{}/chats/{}", api_base_url, workspace_id, chat_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "content": prompt }))
        .send().await?;
    wait_for_agent_completion(&mut event_stream).await?;

    // --- TURN 3: Global Refactor ---
    println!("👉 TURN 3: Global Refactor (edit-many)");
    let prompt = "Logging refactor time. YOU MUST use the 'edit-many' tool specifically to replace all occurrences of 'DEBUG:' with 'LOG:' in '/src/main.rs'. DO NOT use 'write'; use 'edit-many'.";
    println!("💬 PROMPT: {}", prompt);
    client.post(&format!("{}/workspaces/{}/chats/{}", api_base_url, workspace_id, chat_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "content": prompt }))
        .send().await?;
    wait_for_agent_completion(&mut event_stream).await?;

    // --- TURN 4: Cleanup & Reorg ---
    println!("👉 TURN 4: Cleanup & Reorg (mkdir, mv, rm, touch, ls)");
    let prompt = "Final cleanup: 1. Use 'mkdir' to create a directory named '/backup'. 2. Use 'mv' to move 'src/main.rs' to 'src/app.rs'. 3. Use 'rm' to delete 'temp_notes.txt'. 4. Use 'touch' to create an empty file named '/backup/SUCCESS'. 5. List the workspace recursively to verify.";
    println!("💬 PROMPT: {}", prompt);
    client.post(&format!("{}/workspaces/{}/chats/{}", api_base_url, workspace_id, chat_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "content": prompt }))
        .send().await?;
    wait_for_agent_completion(&mut event_stream).await?;

    println!("\n⭐ Example Complete! All tools demonstrated in a stateful multi-turn flow.");
    Ok(())
}
