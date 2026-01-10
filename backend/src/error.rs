use thiserror::Error;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// Import Axum types for HTTP response conversion
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

/// Structured validation errors with field-level error mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValidationErrors {
    Single { field: String, message: String },
    Multiple { fields: HashMap<String, String> },
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationErrors::Single { field, message } => {
                write!(f, "{}: {}", field, message)
            }
            ValidationErrors::Multiple { fields } => {
                let errors: Vec<String> = fields
                    .iter()
                    .map(|(field, message)| format!("{}: {}", field, message))
                    .collect();
                write!(f, "Validation errors: {}", errors.join(", "))
            }
        }
    }
}

/// The custom error type for the application.
#[derive(Debug, Error)]
pub enum Error {
    /// An error originating from the sqlx library.
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// An error originating from IO operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A validation error with field-level details.
    #[error("Validation error: {0}")]
    Validation(ValidationErrors),

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

    /// Token theft detected (stolen refresh token used after rotation).
    #[error("Token theft detected: {0}")]
    TokenTheftDetected(String),

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
/// and returns a JSON response with an error message and error code.
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (body, status) = match self {
            Error::Validation(errors) => {
                let body = match errors {
                    ValidationErrors::Single { field, message } => {
                        serde_json::json!({
                            "error": "Validation failed",
                            "code": "VALIDATION_ERROR",
                            "fields": {
                                field: message
                            }
                        })
                    }
                    ValidationErrors::Multiple { fields } => {
                        serde_json::json!({
                            "error": "Validation failed",
                            "code": "VALIDATION_ERROR",
                            "fields": fields
                        })
                    }
                };
                (body, StatusCode::BAD_REQUEST)
            }
            Error::NotFound(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "NOT_FOUND"
                });
                (body, StatusCode::NOT_FOUND)
            }
            Error::Forbidden(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "FORBIDDEN"
                });
                (body, StatusCode::FORBIDDEN)
            }
            Error::Conflict(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "CONFLICT"
                });
                (body, StatusCode::CONFLICT)
            }
            Error::Authentication(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "AUTHENTICATION_FAILED"
                });
                (body, StatusCode::UNAUTHORIZED)
            }
            Error::InvalidToken(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "INVALID_TOKEN"
                });
                (body, StatusCode::UNAUTHORIZED)
            }
            Error::SessionExpired(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "SESSION_EXPIRED"
                });
                (body, StatusCode::UNAUTHORIZED)
            }
            Error::TokenTheftDetected(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "TOKEN_THEFT"
                });
                (body, StatusCode::FORBIDDEN)
            }
            Error::Sqlx(_) => {
                let body = serde_json::json!({
                    "error": "Database error",
                    "code": "INTERNAL_ERROR"
                });
                (body, StatusCode::INTERNAL_SERVER_ERROR)
            }
            Error::Internal(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "INTERNAL_ERROR"
                });
                (body, StatusCode::INTERNAL_SERVER_ERROR)
            }
            Error::Config(_) => {
                let body = serde_json::json!({
                    "error": "Configuration error",
                    "code": "CONFIG_ERROR"
                });
                (body, StatusCode::INTERNAL_SERVER_ERROR)
            }
            Error::Cache(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "CACHE_ERROR"
                });
                (body, StatusCode::INTERNAL_SERVER_ERROR)
            }
            Error::CacheSerialization(msg) => {
                let body = serde_json::json!({
                    "error": msg,
                    "code": "CACHE_ERROR"
                });
                (body, StatusCode::INTERNAL_SERVER_ERROR)
            }
            Error::Io(_) => {
                let body = serde_json::json!({
                    "error": "IO error",
                    "code": "INTERNAL_ERROR"
                });
                (body, StatusCode::INTERNAL_SERVER_ERROR)
            }
        };

        (status, Json(body)).into_response()
    }
}
