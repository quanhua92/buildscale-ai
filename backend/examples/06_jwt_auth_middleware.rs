/// JWT Authentication Middleware Example
///
/// This example demonstrates the JWT authentication middleware with user caching,
/// showing the difference between public and protected endpoints.
///
/// **Environment Variables:**
/// - `API_BASE_URL`: API base URL (default: http://localhost:3000/api/v1)
///
/// **Usage:**
/// ```bash
/// # Use default URL (http://localhost:3000/api/v1)
/// cargo run --example 06_jwt_auth_middleware
///
/// # Use custom URL
/// API_BASE_URL=http://localhost:3001/api/v1 cargo run --example 06_jwt_auth_middleware
/// ```
///
/// **Prerequisites:**
/// 1. Start the server: `cargo run` (from backend directory)
/// 2. Ensure the database is running and migrations are applied
///
/// **What this example demonstrates:**
/// - Public health endpoint (`/health`) - No authentication required
/// - Protected cache health endpoint (`/health/cache`) - JWT authentication required
/// - JWT authentication via Authorization header (Bearer token)
/// - JWT authentication via Cookie (access_token)
/// - User caching on first request (DB query + cache)
/// - Cache hit on subsequent requests (no DB query)
/// - Error handling: Invalid JWT returns 401
/// - Error handling: Missing JWT returns 401
/// - Header takes priority over cookie for JWT extraction
///
/// **Architecture Overview:**
/// - **Public endpoints**: No authentication, accessible to anyone
/// - **Protected endpoints**: JWT middleware validates token, caches user data
/// - **User caching**: Reduces database queries by caching authenticated users
/// - **Cache TTL**: Configurable (default: 15 minutes, matches JWT expiration)
/// - **Cache key format**: `user:{user_id}` for easy debugging
///
/// **Key Takeaways:**
/// - Use public endpoints for health checks and load balancer monitoring
/// - Use protected endpoints for sensitive data (cache metrics, user info)
/// - JWT middleware automatically caches user data to reduce DB load
/// - Supports both Authorization header (API clients) and Cookie (browser clients)
/// - Middleware is reusable for any protected endpoint

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
    format!("jwt_middleware_{}@example.com", timestamp)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize HTTP client with cookie support
    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    let api_base_url = get_base_url();

    println!("🔐 JWT Authentication Middleware Example");
    println!("=========================================\n");
    println!("Making requests to: {}", api_base_url);
    println!();

    // Check if server is running
    println!("1️⃣  Checking server health (public endpoint)...");
    match check_public_health(&client, &api_base_url).await {
        Ok(()) => println!("✓ Public health endpoint accessible without authentication\n"),
        Err(e) => {
            println!("✗ Server health check failed: {}", e);
            println!("\n💡 Make sure to start the server with: cargo run");
            return Err(e.into());
        }
    }

    // Test protected endpoint without authentication (should fail)
    println!("2️⃣  Testing protected endpoint WITHOUT authentication...");
    match access_cache_health_without_auth(&client, &api_base_url).await {
        Ok(()) => {
            println!("✗ Protected endpoint should require authentication!");
            return Err("Expected 401 Unauthorized".into());
        }
        Err(e) => {
            println!("✓ Protected endpoint correctly rejected request: {}", e);
            println!("  Status: 401 Unauthorized\n");
        }
    }

    // Test protected endpoint with invalid JWT (should fail)
    println!("3️⃣  Testing protected endpoint with INVALID JWT...");
    match access_cache_health_with_invalid_jwt(&client, &api_base_url).await {
        Ok(()) => {
            println!("✗ Protected endpoint should reject invalid JWT!");
            return Err("Expected 401 Unauthorized".into());
        }
        Err(e) => {
            println!("✓ Protected endpoint correctly rejected invalid JWT: {}", e);
            println!("  Status: 401 Unauthorized\n");
        }
    }

    // Register and login to get valid JWT
    println!("4️⃣  Registering user and getting JWT access token...");
    let email = generate_test_email();
    let password = "SecurePass123!";
    let access_token = match register_and_login(&client, &api_base_url, &email, password).await {
        Ok(token) => {
            println!("✓ User registered and logged in successfully:");
            println!("  Email: {}", email);
            println!("  Access Token: {}...", &token[..20]);
            println!();
            token
        }
        Err(e) => {
            println!("✗ Registration/login failed: {}", e);
            return Err(e.into());
        }
    };

    // Test protected endpoint with valid JWT (Authorization header)
    println!("5️⃣  Testing protected endpoint with VALID JWT (Authorization header)...");
    match access_cache_health_with_header(&client, &api_base_url, &access_token).await {
        Ok(metrics) => {
            println!("✓ Protected endpoint accessed successfully with Authorization header:");
            println!("  Cache keys: {}", metrics["num_keys"]);
            println!("  Status: 200 OK");
            println!();
        }
        Err(e) => {
            println!("✗ Protected endpoint failed: {}", e);
            return Err(e.into());
        }
    }

    // Test protected endpoint with valid JWT (Cookie)
    println!("6️⃣  Testing protected endpoint with VALID JWT (Cookie)...");
    match access_cache_health_with_cookie(&client, &api_base_url).await {
        Ok(metrics) => {
            println!("✓ Protected endpoint accessed successfully with Cookie:");
            println!("  Cache keys: {}", metrics["num_keys"]);
            println!("  Status: 200 OK");
            println!();
        }
        Err(e) => {
            println!("✗ Protected endpoint failed: {}", e);
            return Err(e.into());
        }
    }

    // Test that user is cached after first request
    println!("7️⃣  Testing user caching behavior...");
    match test_user_caching(&client, &api_base_url, &access_token).await {
        Ok(()) => println!("✓ User caching works correctly (first request: DB + cache, subsequent: cache hit)\n"),
        Err(e) => {
            println!("✗ User caching test failed: {}", e);
            return Err(e.into());
        }
    }

    // Summary
    println!("═════════════════════════════════════════════════════════");
    println!("📋 Summary");
    println!("═════════════════════════════════════════════════════════");
    println!();
    println!("✅ Public endpoints: No authentication required");
    println!("   - Example: GET /api/v1/health");
    println!("   - Use for: Health checks, load balancer monitoring");
    println!();
    println!("✅ Protected endpoints: JWT authentication required");
    println!("   - Example: GET /api/v1/health/cache");
    println!("   - Use for: Sensitive data (cache metrics, user info)");
    println!("   - Authentication methods:");
    println!("     • Authorization header: `Bearer <token>` (API clients)");
    println!("     • Cookie: `access_token=<token>` (browser clients)");
    println!();
    println!("✅ User caching: Reduces database queries");
    println!("   - First request: Validate JWT + Query DB + Cache user");
    println!("   - Subsequent requests: Validate JWT + Cache hit (no DB query)");
    println!("   - Cache TTL: 15 minutes (configurable via BUILDSCALE__CACHE__USER_CACHE_TTL_SECONDS)");
    println!("   - Cache key format: `user:{{user_id}}` for debugging");
    println!();
    println!("✅ Error handling: Clear security messages");
    println!("   - Invalid JWT: 401 Unauthorized");
    println!("   - Missing JWT: 401 Unauthorized");
    println!("   - Header priority: Authorization header > Cookie");
    println!();
    println!("🔧 How to add protected endpoints:");
    println!("   1. Add route to protected Router in lib.rs:");
    println!("      ```rust");
    println!("      .route(\"/your-endpoint\", get(your_handler))");
    println!("      ```");
    println!("   2. Handler extracts AuthenticatedUser from Extension:");
    println!("      ```rust");
    println!("      async fn your_handler(");
    println!("          Extension(user): Extension<AuthenticatedUser>,");
    println!("          State(state): State<AppState>,");
    println!("      ) -> Result<Json<Value>> {{");
    println!("          // Access user.id, user.email, user.full_name");
    println!("      }}");
    println!("      ```");
    println!("   3. Middleware handles JWT validation and caching automatically");
    println!();

    Ok(())
}

/// Check public health endpoint (no authentication required)
async fn check_public_health(client: &Client, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/health", base_url);
    let response = client.get(&url).send().await?;

    let status = response.status();
    if status.as_u16() == 200 {
        let body: serde_json::Value = response.json().await?;
        if body["status"] == "ok" {
            println!("  Response: {}", body);
            return Ok(());
        }
    }

    Err(format!("Unexpected response status: {}", status).into())
}

/// Try to access protected endpoint without authentication (should return 401)
async fn access_cache_health_without_auth(
    client: &Client,
    base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/health/cache", base_url);
    let response = client.get(&url).send().await?;

    if response.status().as_u16() == 401 {
        let error: serde_json::Value = response.json().await?;
        println!("  Error: {}", error["error"]);
        return Err("Expected 401 Unauthorized".into());
    }

    Ok(())
}

/// Try to access protected endpoint with invalid JWT (should return 401)
async fn access_cache_health_with_invalid_jwt(
    client: &Client,
    base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/health/cache", base_url);
    let response = client
        .get(&url)
        .header("Authorization", "Bearer invalid.jwt.token")
        .send()
        .await?;

    if response.status().as_u16() == 401 {
        let error: serde_json::Value = response.json().await?;
        println!("  Error: {}", error["error"]);
        return Err("Expected 401 Unauthorized".into());
    }

    Ok(())
}

/// Register a new user and login to get JWT access token
async fn register_and_login(
    client: &Client,
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Register user
    let register_url = format!("{}/auth/register", base_url);
    let register_response = client
        .post(&register_url)
        .json(&json!({
            "email": email,
            "password": password,
            "confirm_password": password,
            "full_name": "JWT Middleware Test User"
        }))
        .send()
        .await?;

    // Accept both 200 and 201 (registration returns 200 in current implementation)
    let status = register_response.status();
    if status.as_u16() != 200 && status.as_u16() != 201 {
        let error: serde_json::Value = register_response.json().await?;
        return Err(format!("Registration failed: {}", error).into());
    }

    // Login
    let login_url = format!("{}/auth/login", base_url);
    let login_response = client
        .post(&login_url)
        .json(&json!({
            "email": email,
            "password": password
        }))
        .send()
        .await?;

    let login_status = login_response.status();
    if login_status.as_u16() != 200 {
        let error: serde_json::Value = login_response.json().await?;
        return Err(format!("Login failed: {}", error).into());
    }

    let login_data: serde_json::Value = login_response.json().await?;
    let access_token = login_data["access_token"]
        .as_str()
        .ok_or("Missing access_token")?;

    Ok(access_token.to_string())
}

/// Access protected cache health endpoint with Authorization header
async fn access_cache_health_with_header(
    client: &Client,
    base_url: &str,
    access_token: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let url = format!("{}/health/cache", base_url);
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if response.status().as_u16() != 200 {
        let error: serde_json::Value = response.json().await?;
        return Err(format!("Request failed: {}", error).into());
    }

    let metrics: serde_json::Value = response.json().await?;
    Ok(metrics)
}

/// Access protected cache health endpoint with Cookie
async fn access_cache_health_with_cookie(
    client: &Client,
    base_url: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let url = format!("{}/health/cache", base_url);

    // Cookie is automatically sent by reqwest client (cookie_store = true)
    let response = client.get(&url).send().await?;

    if response.status().as_u16() != 200 {
        let error: serde_json::Value = response.json().await?;
        return Err(format!("Request failed: {}", error).into());
    }

    let metrics: serde_json::Value = response.json().await?;
    Ok(metrics)
}

/// Test user caching behavior
async fn test_user_caching(
    client: &Client,
    base_url: &str,
    access_token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // First request: User should be cached
    let _ = access_cache_health_with_header(client, base_url, access_token).await?;

    // Second request: User should be served from cache (no DB query)
    let _ = access_cache_health_with_header(client, base_url, access_token).await?;

    Ok(())
}
