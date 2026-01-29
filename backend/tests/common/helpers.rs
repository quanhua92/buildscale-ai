//! Shared test helper functions
//!
//! This module provides common utility functions used across multiple test files,
//! reducing code duplication and ensuring consistent test patterns.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::TestApp;

/// Generates a unique test email using nanosecond timestamp
///
/// # Returns
/// A unique email address in the format `test_{timestamp}@example.com`
///
/// # Example
/// ```no_run
/// let email = generate_test_email();
/// // Returns something like "test_1234567890123456789@example.com"
/// ```
pub fn generate_test_email() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let random: u32 = rand::random();
    format!("test_{}_{}@example.com", timestamp, random)
}

/// Registers and logs in a user, returning the access token
///
/// This helper function performs two operations:
/// 1. Registers a new user with a unique email
/// 2. Logs in the user and returns the JWT access token
///
/// # Arguments
/// * `app` - Reference to the test application
///
/// # Returns
/// The JWT access token as a String
///
/// # Example
/// ```no_run
/// let app = TestApp::new().await;
/// let token = register_and_login(&app).await;
/// ```
pub async fn register_and_login(app: &TestApp) -> String {
    let email = generate_test_email();

    // Register user
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

    // Login to get access token
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

    assert_eq!(login_response.status(), 200);

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    login_body["access_token"].as_str().unwrap().to_string()
}

/// Creates a workspace as an authenticated user
///
/// # Arguments
/// * `app` - Reference to the test application
/// * `token` - JWT access token for authentication
/// * `name` - Name for the workspace
///
/// # Returns
/// The workspace ID as a String
///
/// # Example
/// ```no_run
/// let app = TestApp::new().await;
/// let token = register_and_login(&app).await;
/// let workspace_id = create_workspace(&app, &token, "My Workspace").await;
/// ```
pub async fn create_workspace(app: &TestApp, token: &str, name: &str) -> String {
    let response = app
        .client
        .post(&app.url("/api/v1/workspaces"))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": name
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    body["workspace"]["id"].as_str().unwrap().to_string()
}
