/// OpenAI REST API Integration Test
///
/// Tests the backend's REST API chat endpoint end-to-end
///
/// PREREQUISITE: Start the backend first:
///   docker compose -f docker-compose.local.yml up -d --build
///
/// Then run: cargo run --example api_chat_schema

use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    println!("=== OpenAI REST API Integration Test ===\n");

    // Get API URL from environment or use default
    let api_base = env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000/api/v1".to_string());

    println!("API Base: {}\n", api_base);

    // Create HTTP client
    let client = reqwest::Client::new();

    // Step 1: Login with default test account
    println!("--- Step 1: Login ---");
    let login_request = serde_json::json!({
        "email": "test@example.com",
        "password": "SecureTestPass123!@#"
    });

    let login_url = format!("{}/auth/login", api_base);
    println!("POST {}", login_url);

    let mut login_response = client
        .post(&login_url)
        .json(&login_request)
        .send()
        .await?;

    let login_status = login_response.status();
    let mut login_body = login_response.text().await?;
    println!("Status: {}", login_status);
    println!("Response: {}\n", login_body);

    if !login_status.is_success() {
        // If login fails, try to register the user first
        println!("Login failed, trying to register user...\n");
        let register_request = serde_json::json!({
            "email": "test@example.com",
            "password": "SecureTestPass123!@#",
            "confirm_password": "SecureTestPass123!@#",
            "full_name": "Test User"
        });

        let register_url = format!("{}/auth/register", api_base);
        println!("POST {}", register_url);

        let register_response = client
            .post(&register_url)
            .json(&register_request)
            .send()
            .await?;

        let register_status = register_response.status();
        let register_body = register_response.text().await?;
        println!("Status: {}", register_status);
        println!("Response: {}\n", register_body);

        if !register_status.is_success() {
            println!("✗ FAILED: Could not register user");
            return Err(register_body.into());
        }

        // Now try login again
        println!("Retrying login...\n");
        login_response = client
            .post(&login_url)
            .json(&login_request)
            .send()
            .await?;

        let login_status = login_response.status();
        login_body = login_response.text().await?;
        println!("Status: {}", login_status);
        println!("Response: {}\n", login_body);

        if !login_status.is_success() {
            println!("✗ FAILED: Could not login after registration");
            return Err(login_body.into());
        }
    }

    // Parse login response to get access token
    let login_data: serde_json::Value = serde_json::from_str(&login_body)?;
    let access_token = login_data["access_token"]
        .as_str()
        .ok_or("Missing access_token")?;

    println!("✓ Login successful!\n");

    // Step 2: List workspaces and get or create one
    println!("--- Step 2: Get/Create Workspace ---");
    let list_workspaces_url = format!("{}/workspaces", api_base);
    println!("GET {}", list_workspaces_url);

    let list_response = client
        .get(&list_workspaces_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    let list_status = list_response.status();
    let list_body = list_response.text().await?;
    println!("Status: {}", list_status);
    println!("Response: {}\n", list_body);

    let workspace_id = if list_status.is_success() {
        // Parse workspaces list
        let list_data: serde_json::Value = serde_json::from_str(&list_body)?;
        if let Some(workspaces) = list_data["workspaces"].as_array() {
            if !workspaces.is_empty() {
                // Use the first workspace
                let first_workspace = &workspaces[0];
                let id = first_workspace["id"].as_str().unwrap_or("");
                println!("Using existing workspace: {}\n", id);
                id.to_string()
            } else {
                // Create a new workspace
                println!("No workspaces found, creating one...\n");
                create_workspace(&client, &api_base, access_token).await?
            }
        } else {
            // Create a new workspace
            println!("Invalid workspaces response, creating one...\n");
            create_workspace(&client, &api_base, access_token).await?
        }
    } else {
        // Create a new workspace
        println!("Failed to list workspaces, creating one...\n");
        create_workspace(&client, &api_base, access_token).await?
    };

    // Step 3: Create a new chat
    println!("--- Step 3: Create Chat ---");
    let chat_id = create_chat(&client, &api_base, access_token, &workspace_id).await?;

    // Step 4: Send a message to the chat
    println!("--- Step 4: Send Message to Chat ---");
    println!("Sending: \"Say hello\"\n");

    let request = serde_json::json!({
        "content": "Say hello"
    });

    let url = format!("{}/workspaces/{}/chats/{}", api_base, workspace_id, chat_id);

    println!("POST {}", url);
    println!("Request body: {}\n", serde_json::to_string_pretty(&request)?);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    println!("Status: {}", status);
    println!("Headers:\n{:?}", response.headers());

    let body = response.text().await?;
    println!("\nResponse body:\n{}", body);

    if status.is_success() {
        println!("\n✓ SUCCESS: Message sent successfully!");
        Ok(())
    } else {
        println!("\n✗ FAILED: API returned error status");
        Err(body.into())
    }
}

async fn create_workspace(
    client: &reqwest::Client,
    api_base: &str,
    access_token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let create_request = serde_json::json!({
        "name": "Test Workspace"
    });

    let create_url = format!("{}/workspaces", api_base);
    println!("POST {}", create_url);

    let create_response = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&create_request)
        .send()
        .await?;

    let create_status = create_response.status();
    let create_body = create_response.text().await?;
    println!("Status: {}", create_status);
    println!("Response: {}\n", create_body);

    if !create_status.is_success() {
        println!("✗ FAILED: Could not create workspace");
        return Err(create_body.into());
    }

    let create_data: serde_json::Value = serde_json::from_str(&create_body)?;
    let workspace_id = create_data["workspace"]["id"]
        .as_str()
        .ok_or("Missing workspace id")?;

    println!("✓ Workspace created: {}\n", workspace_id);
    Ok(workspace_id.to_string())
}

async fn create_chat(
    client: &reqwest::Client,
    api_base: &str,
    access_token: &str,
    workspace_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let create_request = serde_json::json!({
        "goal": "Test chat for API integration testing"
    });

    let create_url = format!("{}/workspaces/{}/chats", api_base, workspace_id);
    println!("POST {}", create_url);

    let create_response = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&create_request)
        .send()
        .await?;

    let create_status = create_response.status();
    let create_body = create_response.text().await?;
    println!("Status: {}", create_status);
    println!("Response: {}\n", create_body);

    if !create_status.is_success() {
        println!("✗ FAILED: Could not create chat");
        return Err(create_body.into());
    }

    let create_data: serde_json::Value = serde_json::from_str(&create_body)?;
    let chat_id = create_data["chat_id"]
        .as_str()
        .ok_or("Missing chat id")?;

    println!("✓ Chat created: {}\n", chat_id);
    Ok(chat_id.to_string())
}
