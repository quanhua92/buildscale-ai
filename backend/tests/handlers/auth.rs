use crate::common::TestApp;

/// Helper function to generate a unique test email
fn generate_test_email() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("test_{}@example.com", timestamp)
}

#[tokio::test]
async fn test_register_endpoint_returns_200_on_success() {
    let app = TestApp::new().await;

    let response = app
        .client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": generate_test_email(),
            "password": "password123",
            "confirm_password": "password123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_register_endpoint_returns_json_content_type() {
    let app = TestApp::new().await;

    let response = app
        .client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": generate_test_email(),
            "password": "password123",
            "confirm_password": "password123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_register_endpoint_returns_user_object() {
    let app = TestApp::new().await;

    let response = app
        .client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": generate_test_email(),
            "password": "password123",
            "confirm_password": "password123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["user"].is_object());
    assert!(body["user"]["id"].is_string());
    assert!(body["user"]["email"].is_string());
}

#[tokio::test]
async fn test_register_endpoint_returns_400_on_password_mismatch() {
    let app = TestApp::new().await;

    let response = app
        .client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": generate_test_email(),
            "password": "password123",
            "confirm_password": "different"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_login_endpoint_returns_200_on_valid_credentials() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // First register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123",
            "confirm_password": "password123"
        }))
        .send()
        .await
        .unwrap();

    // Then login with the same credentials
    let response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_login_endpoint_sets_access_token_cookie() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123",
            "confirm_password": "password123"
        }))
        .send()
        .await
        .unwrap();

    // Login
    let response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Check Set-Cookie headers
    let cookies = response.headers().get_all("set-cookie");
    let cookie_strs: Vec<&str> = cookies
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();

    // Verify access_token cookie is set
    assert!(
        cookie_strs.iter().any(|c| c.contains("access_token=")),
        "access_token cookie should be set"
    );
}

#[tokio::test]
async fn test_login_endpoint_sets_refresh_token_cookie() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123",
            "confirm_password": "password123"
        }))
        .send()
        .await
        .unwrap();

    // Login
    let response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Check Set-Cookie headers
    let cookies = response.headers().get_all("set-cookie");
    let cookie_strs: Vec<&str> = cookies
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();

    // Verify refresh_token cookie is set
    assert!(
        cookie_strs.iter().any(|c| c.contains("refresh_token=")),
        "refresh_token cookie should be set"
    );
}

#[tokio::test]
async fn test_login_endpoint_returns_tokens_in_json_body() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123",
            "confirm_password": "password123"
        }))
        .send()
        .await
        .unwrap();

    // Login
    let response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Check JSON body contains tokens for API clients
    let body: serde_json::Value = response.json().await.unwrap();

    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
    assert!(body["user"].is_object());
    assert!(body["access_token_expires_at"].is_string());
    assert!(body["refresh_token_expires_at"].is_string());
}

#[tokio::test]
async fn test_login_endpoint_returns_401_on_invalid_credentials() {
    let app = TestApp::new().await;

    // Try to login without registering
    let response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": generate_test_email(),
            "password": "wrongpassword"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_cookies_have_http_only_flag() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123",
            "confirm_password": "password123"
        }))
        .send()
        .await
        .unwrap();

    // Login
    let response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();

    // Check Set-Cookie headers have HttpOnly flag
    let cookies = response.headers().get_all("set-cookie");
    let cookie_strs: Vec<&str> = cookies
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();

    for cookie in cookie_strs {
        if cookie.contains("access_token=") || cookie.contains("refresh_token=") {
            assert!(
                cookie.contains("HttpOnly"),
                "Cookie should have HttpOnly flag for XSS protection"
            );
        }
    }
}

#[tokio::test]
async fn test_cookies_have_same_site_lax_flag() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123",
            "confirm_password": "password123"
        }))
        .send()
        .await
        .unwrap();

    // Login
    let response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();

    // Check Set-Cookie headers have SameSite=Lax flag
    let cookies = response.headers().get_all("set-cookie");
    let cookie_strs: Vec<&str> = cookies
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();

    for cookie in cookie_strs {
        if cookie.contains("access_token=") || cookie.contains("refresh_token=") {
            assert!(
                cookie.contains("SameSite=Lax"),
                "Cookie should have SameSite=Lax flag for CSRF protection"
            );
        }
    }
}
