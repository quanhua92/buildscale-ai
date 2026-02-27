use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user session stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new user session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUserSession {
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

/// Data for updating an existing user session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserSession {
    pub expires_at: Option<DateTime<Utc>>,
}

/// A revoked refresh token for theft detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokedRefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub revoked_at: DateTime<Utc>,
    pub reason: String,
}
