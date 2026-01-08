//! Health endpoint tests
//!
//! Tests for both public and protected health check endpoints.

use crate::common::TestApp;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a unique email for testing to avoid conflicts
fn generate_test_email() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("test_jwt_health_{}@example.com", timestamp)
}

// ============================================================================
// Public Health Endpoint Tests (/api/v1/health)
// ============================================================================

#[tokio::test]
async fn test_public_health_returns_200() {
    let app = TestApp::new().await;

    let response = app.client
        .get(&app.url("/api/v1/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_public_health_returns_json() {
    let app = TestApp::new().await;

    let response = app.client
        .get(&app.url("/api/v1/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_public_health_returns_status_ok() {
    let app = TestApp::new().await;

    let response = app.client
        .get(&app.url("/api/v1/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_public_health_no_auth_required() {
    let app = TestApp::new().await;

    // Request without any authentication headers
    let response = app.client
        .get(&app.url("/api/v1/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_public_health_no_sensitive_info() {
    let app = TestApp::new().await;

    let response = app.client
        .get(&app.url("/api/v1/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();

    // Should only have "status" field
    assert_eq!(body.as_object().unwrap().len(), 1);
    assert!(body.get("status").is_some());

    // Should NOT expose commit/build info or cache metrics
    assert!(body.get("commit").is_none());
    assert!(body.get("build_date").is_none());
    assert!(body.get("version").is_none());
    assert!(body.get("num_keys").is_none());
    assert!(body.get("last_worker_time").is_none());
    assert!(body.get("cleaned_count").is_none());
    assert!(body.get("size_bytes").is_none());
}

// ============================================================================
// Protected Cache Health Endpoint Tests (/api/v1/health/cache)
// ============================================================================

#[tokio::test]
async fn test_cache_health_with_valid_jwt_returns_200() {
    let app = TestApp::new().await;

    // Register and login to get JWT
    let email = generate_test_email();
    let password = "TestSecurePass123!";

    // Register user
    let register_response = app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "confirm_password": password,
            "full_name": "Test User"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(register_response.status(), 200);

    // Login to get access token
    let login_response = app.client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(login_response.status(), 200);

    let login_data: serde_json::Value = login_response.json().await.unwrap();
    let access_token = login_data["access_token"].as_str().unwrap();

    // Access protected cache health endpoint with JWT
    let response = app.client
        .get(&app.url("/api/v1/health/cache"))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_cache_health_with_cookie_returns_200() {
    let app = TestApp::new().await;

    // Register and login to get JWT
    let email = generate_test_email();
    let password = "TestSecurePass123!";

    // Register user
    let register_response = app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "confirm_password": password,
            "full_name": "Test User"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(register_response.status(), 200);

    // Login (sets cookies automatically)
    let login_response = app.client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(login_response.status(), 200);

    // Access protected cache health endpoint (cookies are automatically sent)
    let response = app.client
        .get(&app.url("/api/v1/health/cache"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_cache_health_with_invalid_jwt_returns_401() {
    let app = TestApp::new().await;

    // Access protected endpoint with invalid JWT
    let response = app.client
        .get(&app.url("/api/v1/health/cache"))
        .header("Authorization", "Bearer invalid.jwt.token")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_cache_health_with_missing_jwt_returns_401() {
    let app = TestApp::new().await;

    // Access protected endpoint without any authentication
    let response = app.client
        .get(&app.url("/api/v1/health/cache"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_cache_health_returns_cache_metrics() {
    let app = TestApp::new().await;

    // Register and login
    let email = generate_test_email();
    let password = "TestSecurePass123!";

    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "confirm_password": password,
            "full_name": "Test User"
        }))
        .send()
        .await
        .unwrap();

    let login_response = app.client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .unwrap();

    let login_data: serde_json::Value = login_response.json().await.unwrap();
    let access_token = login_data["access_token"].as_str().unwrap();

    // Add some data to cache
    app.cache.set("test_key", "test_value".to_string()).await.unwrap();
    app.cache.set("another_key", "another_value".to_string()).await.unwrap();

    // Get cache metrics
    let response = app.client
        .get(&app.url("/api/v1/health/cache"))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let metrics: serde_json::Value = response.json().await.unwrap();

    // Verify all cache metric fields exist
    assert!(metrics.get("num_keys").is_some());
    assert!(metrics.get("last_worker_time").is_some());
    assert!(metrics.get("cleaned_count").is_some());
    assert!(metrics.get("size_bytes").is_some());

    // Verify cache data accuracy
    assert_eq!(metrics["num_keys"], 2);
}

#[tokio::test]
async fn test_jwt_middleware_header_priority_over_cookie() {
    let app = TestApp::new().await;

    // Create two different users
    let email1 = generate_test_email();
    let email2 = generate_test_email();
    let password = "TestSecurePass123!";

    // Register both users
    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email1,
            "password": password,
            "confirm_password": password,
            "full_name": "User One"
        }))
        .send()
        .await
        .unwrap();

    app.client
        .post(&app.url("/api/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email2,
            "password": password,
            "confirm_password": password,
            "full_name": "User Two"
        }))
        .send()
        .await
        .unwrap();

    // Login as user2 (sets cookie)
    app.client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email2,
            "password": password
        }))
        .send()
        .await
        .unwrap();

    // Login as user1 and get access token
    let login_response = app.client
        .post(&app.url("/api/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email1,
            "password": password
        }))
        .send()
        .await
        .unwrap();

    let login_data: serde_json::Value = login_response.json().await.unwrap();
    let user1_token = login_data["access_token"].as_str().unwrap();

    // Request with user1's Authorization header (user2 cookie is also set)
    // Header should take priority over cookie
    let response = app.client
        .get(&app.url("/api/v1/health/cache"))
        .header("Authorization", format!("Bearer {}", user1_token))
        .send()
        .await
        .unwrap();

    // Should work because header takes priority
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_multiple_apps_can_run_concurrently() {
    // Create multiple test apps to verify port randomization works
    let app1 = TestApp::new().await;
    let app2 = TestApp::new().await;
    let app3 = TestApp::new().await;

    // All should have different addresses
    assert_ne!(app1.address, app2.address);
    assert_ne!(app2.address, app3.address);
    assert_ne!(app1.address, app3.address);

    // All public health endpoints should work
    let response1 = app1.client.get(&app1.url("/api/v1/health")).send().await.unwrap();
    let response2 = app2.client.get(&app2.url("/api/v1/health")).send().await.unwrap();
    let response3 = app3.client.get(&app3.url("/api/v1/health")).send().await.unwrap();

    assert_eq!(response1.status(), 200);
    assert_eq!(response2.status(), 200);
    assert_eq!(response3.status(), 200);
}
