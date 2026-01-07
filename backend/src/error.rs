use thiserror::Error;

// Import Axum types for HTTP response conversion
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

/// The custom error type for the application.
#[derive(Debug, Error)]
pub enum Error {
    /// An error originating from the sqlx library.
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// A validation error.
    #[error("Validation error: {0}")]
    Validation(String),

    /// A not found error (resource does not exist).
    #[error("Not found: {0}")]
    NotFound(String),

    /// A forbidden error (user lacks permission).
    #[error("Access forbidden: {0}")]
    Forbidden(String),

    /// A conflict error (resource already exists).
    #[error("Conflict: {0}")]
    Conflict(String),

    /// An authentication error (invalid credentials).
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// A session expired error.
    #[error("Session expired: {0}")]
    SessionExpired(String),

    /// An invalid session token error.
    #[error("Invalid session token: {0}")]
    InvalidToken(String),

    /// An internal server error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// A configuration error.
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// A cache operation error.
    #[error("Cache error: {0}")]
    Cache(String),

    /// A cache serialization error (for Redis compatibility).
    #[error("Cache serialization error: {0}")]
    CacheSerialization(String),
}

/// A type alias for `Result<T, Error>` to simplify function signatures.
pub type Result<T> = std::result::Result<T, Error>;

/// Convert custom Error to HTTP response
///
/// This implementation maps each error variant to an appropriate HTTP status code
/// and returns a JSON response with an error message.
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Error::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Error::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            Error::Conflict(msg) => (StatusCode::CONFLICT, msg),
            Error::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg),
            Error::InvalidToken(msg) => (StatusCode::UNAUTHORIZED, msg),
            Error::SessionExpired(msg) => (StatusCode::UNAUTHORIZED, msg),
            Error::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()),
            Error::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error".to_string()),
            Error::Cache(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::CacheSerialization(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}
