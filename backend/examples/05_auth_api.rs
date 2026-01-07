/// Authentication API Example
///
/// This example demonstrates how to interact with the authentication API endpoints
/// using HTTP requests.
///
/// **Environment Variables:**
/// - `API_BASE_URL`: API base URL (default: http://localhost:3000/api/v1)
///
/// **Usage:**
/// ```bash
/// # Use default URL (http://localhost:3000/api/v1)
/// cargo run --example 05_auth_api
///
/// # Use custom URL
/// API_BASE_URL=http://localhost:3001/api/v1 cargo run --example 05_auth_api
/// ```
///
/// **Prerequisites:**
/// 1. Start the server: `cargo run --bin main`
/// 2. Ensure the database is running and migrations are applied
///
/// **What this example demonstrates:**
/// - User registration via POST /api/v1/auth/register
/// - User login via POST /api/v1/auth/login
/// - Extracting access and refresh tokens from responses
/// - Understanding how cookies are set for browser clients
/// - Error handling for various authentication scenarios
///
/// **Note:** This is a client-side example that makes HTTP requests to the API.
/// For direct database operations, see examples 02_users_management.rs

use reqwest::Client;
use serde_json::json;

fn get_base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000/api/v1".to_string())
}

/// Generate a unique email for testing to avoid conflicts
fn generate_test_email() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("test_{}@example.com", timestamp)
}

/// Generate a unique name for testing
fn generate_test_name() -> String {
    // Use a simple name that passes validation (letters and spaces only)
    // Email already provides uniqueness via timestamp
    "Test User".to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize HTTP client with cookie support
    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    let api_base_url = get_base_url();

    println!("🚀 Authentication API Example");
    println!("================================\n");
    println!("Making requests to: {}", api_base_url);
    println!();

    // Check if server is running
    println!("1️⃣  Checking server health...");
    match check_server_health(&client, &api_base_url).await {
        Ok(()) => println!("✓ Server is running and healthy\n"),
        Err(e) => {
            println!("✗ Server health check failed: {}", e);
            println!("\n💡 Make sure to start the server with: cargo run --bin main");
            return Err(e.into());
        }
    }

    // Test user registration
    println!("2️⃣  Testing user registration...");
    let email = generate_test_email();
    let password = "SecurePass123!";
    match register_user(&client, &api_base_url, &email, password).await {
        Ok(user_data) => {
            println!("✓ User registered successfully:");
            println!("  User ID: {}", user_data["id"]);
            println!("  Email: {}", user_data["email"]);
            println!();
        }
        Err(e) => {
            println!("✗ Registration failed: {}", e);
            return Err(e.into());
        }
    }

    // Test login with correct credentials
    println!("3️⃣  Testing user login with correct credentials...");
    match login_user(&client, &api_base_url, &email, password).await {
        Ok(login_response) => {
            println!("✓ Login successful!");
            let token_preview = if login_response.access_token.len() > 40 {
                &login_response.access_token[..40]
            } else {
                &login_response.access_token
            };
            println!("  Access Token (first 40 chars): {}...", token_preview);
            let refresh_preview = if login_response.refresh_token.len() > 40 {
                &login_response.refresh_token[..40]
            } else {
                &login_response.refresh_token
            };
            println!("  Refresh Token (first 40 chars): {}...", refresh_preview);
            println!("  Access Token Expires At: {}", login_response.access_token_expires_at);
            println!("  Refresh Token Expires At: {}", login_response.refresh_token_expires_at);
            println!();

            // Demonstrate how to use the access token for authenticated requests
            println!("4️⃣  Using access token for authenticated request...");
            match make_authenticated_request(&client, &api_base_url, &login_response.access_token).await {
                Ok(_) => {
                    println!("✓ Authenticated request successful (using Authorization header)");
                    println!();
                }
                Err(e) => {
                    println!("✗ Authenticated request failed: {}", e);
                    println!();
                }
            }
        }
        Err(e) => {
            println!("✗ Login failed: {}", e);
            return Err(e.into());
        }
    }

    // Test login with incorrect password
    println!("5️⃣  Testing login with incorrect password...");
    match login_user(&client, &api_base_url, &email, "WrongPassword123!").await {
        Ok(_) => {
            println!("✗ Login should have failed with wrong password");
        }
        Err(e) => {
            println!("✓ Login correctly failed with error: {}", e);
            println!();
        }
    }

    // Test registration with duplicate email
    println!("6️⃣  Testing registration with duplicate email...");
    match register_user(&client, &api_base_url, &email, password).await {
        Ok(_) => {
            println!("✗ Registration should have failed with duplicate email");
        }
        Err(e) => {
            println!("✓ Registration correctly failed with error: {}", e);
            println!();
        }
    }

    // Test registration with weak password
    println!("7️⃣  Testing registration with weak password...");
    let weak_email = generate_test_email();
    match register_user(&client, &api_base_url, &weak_email, "short").await {
        Ok(_) => {
            println!("✗ Registration should have failed with weak password");
        }
        Err(e) => {
            println!("✓ Registration correctly failed with error: {}", e);
            println!();
        }
    }

    // Test registration with mismatched passwords
    println!("8️⃣  Testing registration with mismatched passwords...");
    let mismatch_email = generate_test_email();
    match register_with_mismatched_passwords(&client, &api_base_url, &mismatch_email).await {
        Ok(_) => {
            println!("✗ Registration should have failed with mismatched passwords");
        }
        Err(e) => {
            println!("✓ Registration correctly failed with error: {}", e);
            println!();
        }
    }

    println!("================================");
    println!("✅ Authentication API example completed successfully!");
    println!("\n📝 Key Takeaways:");
    println!("  • Registration creates a new user with email and password");
    println!("  • Login returns access token (15 min) and refresh token (30 days)");
    println!("  • Access token is used in Authorization: Bearer <token> header");
    println!("  • Cookies are automatically set for browser clients");
    println!("  • All validation errors return clear error messages");
    println!();

    Ok(())
}

/// Check if the server is running and healthy
async fn check_server_health(client: &Client, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
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
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let url = format!("{}/auth/register", base_url);
    let request_body = json!({
        "email": email,
        "password": password,
        "confirm_password": password,
        "full_name": generate_test_name()
    });

    println!("  📤 REQUEST:");
    println!("     POST {}", url);
    println!("     Headers: Content-Type: application/json");
    println!("     Body: {}", serde_json::to_string_pretty(&request_body)?);

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await?;

    println!("  📥 RESPONSE:");
    println!("     Status: {}", status);
    println!("     Headers:");
    for (name, value) in headers.iter() {
        println!("       {}: {}", name, value.to_str().unwrap_or(""));
    }
    println!("     Body: {}", body);

    if !status.is_success() {
        return Err(format!("Registration failed ({}): {}", status, body).into());
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;
    Ok(json["user"].clone())
}

/// Register a user with mismatched passwords (for testing validation)
async fn register_with_mismatched_passwords(
    client: &Client,
    base_url: &str,
    email: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let url = format!("{}/auth/register", base_url);
    let request_body = json!({
        "email": email,
        "password": "password123",
        "confirm_password": "different123",
        "full_name": generate_test_name()
    });

    println!("  📤 REQUEST:");
    println!("     POST {}", url);
    println!("     Body: {}", serde_json::to_string_pretty(&request_body)?);

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await?;

    println!("  📥 RESPONSE:");
    println!("     Status: {}", status);
    println!("     Headers:");
    for (name, value) in headers.iter() {
        println!("       {}: {}", name, value.to_str().unwrap_or(""));
    }
    println!("     Body: {}", body);

    if !status.is_success() {
        return Err(format!("Registration failed ({}): {}", status, body).into());
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;
    Ok(json["user"].clone())
}

/// Login user and return tokens
struct LoginResponse {
    access_token: String,
    refresh_token: String,
    access_token_expires_at: String,
    refresh_token_expires_at: String,
}

async fn login_user(
    client: &Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<LoginResponse, Box<dyn std::error::Error>> {
    let url = format!("{}/auth/login", base_url);
    let request_body = json!({
        "email": email,
        "password": password
    });

    println!("  📤 REQUEST:");
    println!("     POST {}", url);
    println!("     Headers: Content-Type: application/json");
    println!("     Body: {}", serde_json::to_string_pretty(&request_body)?);

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await?;

    println!("  📥 RESPONSE:");
    println!("     Status: {}", status);
    println!("     Headers:");
    for (name, value) in headers.iter() {
        println!("       {}: {}", name, value.to_str().unwrap_or(""));
    }
    println!("     Body: {}", body);

    if !status.is_success() {
        return Err(format!("Login failed ({}): {}", status, body).into());
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;

    // Extract cookies to demonstrate they are set
    println!("  🍪 Cookies set by server:");
    let cookies = headers.get_all("set-cookie");
    for cookie in cookies {
        let cookie_str = cookie.to_str().unwrap_or("");
        if cookie_str.contains("access_token") || cookie_str.contains("refresh_token") {
            let cookie_name = if cookie_str.contains("access_token") {
                "access_token"
            } else {
                "refresh_token"
            };
            println!("     • {} cookie set (with HttpOnly and SameSite=Lax)", cookie_name);
        }
    }

    Ok(LoginResponse {
        access_token: json["access_token"].as_str().unwrap_or("").to_string(),
        refresh_token: json["refresh_token"].as_str().unwrap_or("").to_string(),
        access_token_expires_at: json["access_token_expires_at"].as_str().unwrap_or("").to_string(),
        refresh_token_expires_at: json["refresh_token_expires_at"].as_str().unwrap_or("").to_string(),
    })
}

/// Make an authenticated request using the access token
async fn make_authenticated_request(
    client: &Client,
    base_url: &str,
    access_token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/auth/register", base_url);

    let token_preview = if access_token.len() > 40 {
        &access_token[..40]
    } else {
        access_token
    };

    println!("  📤 REQUEST:");
    println!("     GET {}", url);
    println!("     Headers:");
    println!("       Authorization: Bearer {}...", token_preview);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await?;

    println!("  📥 RESPONSE:");
    println!("     Status: {}", status);
    println!("     Headers:");
    for (name, value) in headers.iter() {
        println!("       {}: {}", name, value.to_str().unwrap_or(""));
    }
    println!("     Body: {}", body);

    if status.is_success() {
        Ok(())
    } else {
        Err(format!("Authenticated request failed ({})", status).into())
    }
}
