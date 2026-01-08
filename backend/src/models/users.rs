use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: Option<String>,
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    pub email: String,
    pub password_hash: Option<String>,
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUser {
    pub password_hash: Option<String>,
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUser {
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginUser {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    pub user: User,
    pub access_token: String,           // JWT access token (short-lived, e.g., 15 minutes)
    pub refresh_token: String,          // Session token (long-lived, e.g., 30 days)
    pub access_token_expires_at: DateTime<Utc>,   // When the access token expires
    pub refresh_token_expires_at: DateTime<Utc>,  // When the refresh token expires
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenResult {
    pub access_token: String,                      // New JWT access token
    pub refresh_token: Option<String>,             // New refresh token (rotated), None if within grace period
    pub expires_at: DateTime<Utc>,                 // When the new access token expires
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUserSession {
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserSession {
    pub expires_at: Option<DateTime<Utc>>,
}

/// Revoked refresh token for theft detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokedRefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub revoked_at: DateTime<Utc>,
    pub reason: String,
}
