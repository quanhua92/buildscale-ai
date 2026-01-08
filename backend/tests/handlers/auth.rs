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
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
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
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
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
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
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
            "password": "SecurePass123!",
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
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
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
            "password": "SecurePass123!"
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
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
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
            "password": "SecurePass123!"
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
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
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
            "password": "SecurePass123!"
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
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
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
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Check JSON body contains tokens (login always returns tokens)
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
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
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
            "password": "SecurePass123!"
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
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
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
            "password": "SecurePass123!"
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

// ============================================================================
// REFRESH ENDPOINT TESTS
// ============================================================================

/// Helper function to extract cookie value from response headers
fn extract_cookie_from_response(
    response: &reqwest::Response,
    cookie_name: &str,
) -> String {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|header| header.to_str().ok())
        .find(|header| header.starts_with(&format!("{}=", cookie_name)))
        .map(|header| header.to_string())
        .unwrap_or_default()
}

#[tokio::test]
async fn test_refresh_endpoint_with_authorization_header_returns_200() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    // Login to get refresh token (as API client)
    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Refresh with Authorization header (API client)
    let response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_refresh_endpoint_with_authorization_header_returns_json() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    // Login to get refresh token (as API client)
    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Refresh with Authorization header
    let response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["access_token"].is_string());
    assert!(body["expires_at"].is_string());
}

#[tokio::test]
async fn test_refresh_endpoint_with_authorization_header_does_not_set_cookie() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    // Login to get refresh token (as API client)
    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Refresh with Authorization header (should NOT set cookie)
    let response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Verify NO access_token cookie is set
    let access_cookie = extract_cookie_from_response(&response, "access_token");
    assert!(
        access_cookie.is_empty(),
        "access_token cookie should NOT be set when using Authorization header"
    );
}

#[tokio::test]
async fn test_refresh_endpoint_with_cookie_returns_200() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    // Login to get refresh token (as API client to get tokens in JSON)
    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Refresh with Cookie (browser client)
    let response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Cookie", format!("refresh_token={}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_refresh_endpoint_with_cookie_sets_access_token_cookie() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register a user
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    // Login to get refresh token (as API client to get tokens in JSON)
    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Refresh with Cookie (SHOULD set access_token cookie)
    let response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Cookie", format!("refresh_token={}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Verify access_token cookie IS set
    let access_cookie = extract_cookie_from_response(&response, "access_token");
    assert!(
        !access_cookie.is_empty(),
        "access_token cookie SHOULD be set when using Cookie header"
    );

    // Verify cookie has HttpOnly and SameSite flags
    let cookies = response.headers().get_all("set-cookie");
    let cookie_strs: Vec<&str> = cookies
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();

    let access_token_cookie = cookie_strs
        .iter()
        .find(|c| c.contains("access_token="))
        .expect("access_token cookie should be set");

    assert!(
        access_token_cookie.contains("HttpOnly"),
        "access_token cookie should have HttpOnly flag"
    );
    assert!(
        access_token_cookie.contains("SameSite=Lax"),
        "access_token cookie should have SameSite=Lax flag"
    );
}

#[tokio::test]
async fn test_refresh_endpoint_with_invalid_token_returns_401() {
    let app = TestApp::new().await;

    // Try to refresh with invalid token
    let response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", "Bearer invalid_token_12345")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn test_refresh_endpoint_with_no_token_returns_401() {
    let app = TestApp::new().await;

    // Try to refresh without any token
    let response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn test_refresh_endpoint_with_expired_token_returns_401() {
    let app = TestApp::new().await;

    // Try to refresh with a token that has an expired format
    // This simulates an expired/invalid token without needing database access
    let expired_token = "expired_token_format:invalid_signature";

    let response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", expired_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn test_refresh_endpoint_authorization_header_takes_priority_over_cookie() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register first user to get valid refresh token for Authorization header
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let valid_refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Register second user to get a different refresh token for Cookie
    let email2 = generate_test_email();
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email2,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response2 = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        
        .json(&serde_json::json!({
            "email": email2,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body2: serde_json::Value = login_response2.json().await.unwrap();
    let different_refresh_token = login_body2["refresh_token"].as_str().unwrap();

    // Refresh with BOTH Authorization header and Cookie
    // Authorization header should take priority
    let response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", valid_refresh_token))
        .header("Cookie", format!("refresh_token={}", different_refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Verify NO cookie is set (because Authorization header was used)
    let access_cookie = extract_cookie_from_response(&response, "access_token");
    assert!(
        access_cookie.is_empty(),
        "No cookie should be set when Authorization header takes priority"
    );

    // Verify the access token belongs to the first user (from Authorization header)
    let body: serde_json::Value = response.json().await.unwrap();
    let access_token = body["access_token"].as_str().unwrap();

    // Decode JWT and verify user_id matches first user
    // For now, just verify we got an access token
    assert!(!access_token.is_empty());
}

// ==================== LOGOUT TESTS ====================

#[tokio::test]
async fn test_logout_endpoint_returns_200_on_valid_refresh_token() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register and login to get refresh token
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Logout with Authorization header
    let response = app
        .client
        .post(&app.url("/api/v1/auth/logout"))
        .header("Authorization", format!("Bearer {}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_logout_endpoint_returns_400_on_invalid_token_format() {
    let app = TestApp::new().await;

    // Attempt to logout with invalid token format
    let response = app
        .client
        .post(&app.url("/api/v1/auth/logout"))
        .header("Authorization", "Bearer invalid_refresh_token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_logout_endpoint_returns_401_on_nonexistent_refresh_token() {
    let app = TestApp::new().await;

    // Attempt to logout with valid format but nonexistent token (64 hex chars : 64 hex chars)
    let response = app
        .client
        .post(&app.url("/api/v1/auth/logout"))
        .header("Authorization", "Bearer 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_logout_endpoint_returns_401_on_missing_refresh_token() {
    let app = TestApp::new().await;

    // Attempt to logout without Authorization header
    let response = app
        .client
        .post(&app.url("/api/v1/auth/logout"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_logout_endpoint_clears_cookies() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register and login to get refresh token
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Logout with Cookie (simulating browser client)
    let response = app
        .client
        .post(&app.url("/api/v1/auth/logout"))
        .header("Cookie", format!("refresh_token={}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Verify clear cookies are set (Max-Age=0)
    let access_cookie = extract_cookie_from_response(&response, "access_token");
    let refresh_cookie = extract_cookie_from_response(&response, "refresh_token");

    assert!(
        access_cookie.contains("Max-Age=0") || access_cookie.contains("expires=Thu, 01 Jan 1970"),
        "Access token cookie should be cleared with Max-Age=0 or expired date"
    );
    assert!(
        refresh_cookie.contains("Max-Age=0") || refresh_cookie.contains("expires=Thu, 01 Jan 1970"),
        "Refresh token cookie should be cleared with Max-Age=0 or expired date"
    );
}

#[tokio::test]
async fn test_logout_invalidates_refresh_token() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register and login to get refresh token
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Logout
    let logout_response = app
        .client
        .post(&app.url("/api/v1/auth/logout"))
        .header("Authorization", format!("Bearer {}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(logout_response.status(), 200);

    // Try to use the logged-out token for refresh (should fail)
    let refresh_response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(refresh_response.status(), 401);
}

#[tokio::test]
async fn test_logout_with_authorization_header() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register and login
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Logout with Authorization header (API/mobile client)
    let response = app
        .client
        .post(&app.url("/api/v1/auth/logout"))
        .header("Authorization", format!("Bearer {}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Verify response contains success message
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["message"], "Logout successful");
}

#[tokio::test]
async fn test_logout_with_cookie() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register and login
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Logout with Cookie (browser client)
    let response = app
        .client
        .post(&app.url("/api/v1/auth/logout"))
        .header("Cookie", format!("refresh_token={}", refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Verify response contains success message
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["message"], "Logout successful");
}

#[tokio::test]
async fn test_logout_with_both_header_and_cookie_priority() {
    let app = TestApp::new().await;

    // Register first user to get refresh token
    let email1 = generate_test_email();
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email1,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response1 = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email1,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body1: serde_json::Value = login_response1.json().await.unwrap();
    let refresh_token1 = login_body1["refresh_token"].as_str().unwrap();

    // Register second user to get different refresh token
    let email2 = generate_test_email();
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email2,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response2 = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email2,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body2: serde_json::Value = login_response2.json().await.unwrap();
    let refresh_token2 = login_body2["refresh_token"].as_str().unwrap();

    // Logout with BOTH Authorization header and Cookie
    // Authorization header should take priority (logout user1)
    let response = app
        .client
        .post(&app.url("/api/v1/auth/logout"))
        .header("Authorization", format!("Bearer {}", refresh_token1))
        .header("Cookie", format!("refresh_token={}", refresh_token2))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Verify user1's token is invalidated (from Authorization header)
    let refresh_response1 = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", refresh_token1))
        .send()
        .await
        .unwrap();
    assert_eq!(refresh_response1.status(), 401);

    // Verify user2's token is still valid (not logged out)
    let refresh_response2 = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", refresh_token2))
        .send()
        .await
        .unwrap();
    assert_eq!(refresh_response2.status(), 200);
}

// ============================================================================
// REFRESH TOKEN ROTATION TESTS
// ============================================================================

#[tokio::test]
async fn test_refresh_endpoint_rotates_refresh_token_in_json() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register and login
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let original_refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Refresh with Authorization header (API client)
    let refresh_response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", original_refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(refresh_response.status(), 200);

    let refresh_body: serde_json::Value = refresh_response.json().await.unwrap();
    let new_refresh_token = refresh_body["refresh_token"].as_str().unwrap();

    // Verify new token is different from original (rotation occurred)
    assert_ne!(
        original_refresh_token,
        new_refresh_token,
        "Refresh token should be rotated (new token different from old)"
    );

    // Verify response includes both tokens
    assert!(refresh_body["access_token"].is_string());
    assert!(refresh_body["refresh_token"].is_string());
    assert!(refresh_body["expires_at"].is_string());
}

#[tokio::test]
async fn test_refresh_endpoint_invalidates_old_refresh_token() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register and login
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let original_refresh_token = login_body["refresh_token"].as_str().unwrap();

    // First refresh - get new token
    let refresh_response1 = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", original_refresh_token))
        .send()
        .await
        .unwrap();

    let refresh_body1: serde_json::Value = refresh_response1.json().await.unwrap();
    let new_refresh_token = refresh_body1["refresh_token"].as_str().unwrap();

    // Try to use old token again (should fail - token invalidated)
    let refresh_response2 = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", original_refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(
        refresh_response2.status(),
        401,
        "Old refresh token should be invalidated after rotation"
    );

    // Verify new token still works
    let refresh_response3 = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Authorization", format!("Bearer {}", new_refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(
        refresh_response3.status(),
        200,
        "New refresh token should work"
    );
}

#[tokio::test]
async fn test_refresh_endpoint_with_cookie_rotates_both_cookies() {
    let app = TestApp::new().await;
    let email = generate_test_email();

    // Register and login
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!",
            "confirm_password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app
        .client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "SecurePass123!"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let original_refresh_token = login_body["refresh_token"].as_str().unwrap();

    // Refresh with Cookie (browser client)
    let refresh_response = app
        .client
        .post(&app.url("/api/v1/auth/refresh"))
        .header("Cookie", format!("refresh_token={}", original_refresh_token))
        .send()
        .await
        .unwrap();

    assert_eq!(refresh_response.status(), 200);

    // Verify BOTH cookies are set
    let cookies = refresh_response.headers().get_all("set-cookie");
    let cookie_strs: Vec<&str> = cookies
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();

    let access_cookie = cookie_strs
        .iter()
        .find(|c| c.contains("access_token="))
        .expect("access_token cookie should be set");

    let refresh_cookie = cookie_strs
        .iter()
        .find(|c| c.contains("refresh_token="))
        .expect("refresh_token cookie should be set");

    // Verify cookies are NOT Max-Age=0 (not cleared)
    assert!(!access_cookie.contains("Max-Age=0"));
    assert!(!refresh_cookie.contains("Max-Age=0"));

    // Verify new tokens in JSON response too
    let refresh_body: serde_json::Value = refresh_response.json().await.unwrap();
    assert!(refresh_body["access_token"].is_string());
    assert!(refresh_body["refresh_token"].is_string());
}
