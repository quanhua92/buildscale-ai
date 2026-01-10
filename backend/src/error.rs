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

/// The custom error type for the application.
#[derive(Debug, Error)]
pub enum Error {
    /// An error originating from the sqlx library.
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

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
        let body = match self {
            Error::Validation(errors) => {
                match errors {
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
                }
            }
            Error::NotFound(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "NOT_FOUND"
                })
            }
            Error::Forbidden(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "FORBIDDEN"
                })
            }
            Error::Conflict(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "CONFLICT"
                })
            }
            Error::Authentication(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "AUTHENTICATION_FAILED"
                })
            }
            Error::InvalidToken(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "INVALID_TOKEN"
                })
            }
            Error::SessionExpired(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "SESSION_EXPIRED"
                })
            }
            Error::TokenTheftDetected(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "TOKEN_THEFT"
                })
            }
            Error::Sqlx(_) => {
                serde_json::json!({
                    "error": "Database error",
                    "code": "INTERNAL_ERROR"
                })
            }
            Error::Internal(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "INTERNAL_ERROR"
                })
            }
            Error::Config(_) => {
                serde_json::json!({
                    "error": "Configuration error",
                    "code": "CONFIG_ERROR"
                })
            }
            Error::Cache(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "CACHE_ERROR"
                })
            }
            Error::CacheSerialization(msg) => {
                serde_json::json!({
                    "error": msg,
                    "code": "CACHE_ERROR"
                })
            }
        };

        let status = match self {
            Error::Validation(_) => StatusCode::BAD_REQUEST,
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::Forbidden(_) => StatusCode::FORBIDDEN,
            Error::Conflict(_) => StatusCode::CONFLICT,
            Error::Authentication(_) => StatusCode::UNAUTHORIZED,
            Error::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            Error::SessionExpired(_) => StatusCode::UNAUTHORIZED,
            Error::TokenTheftDetected(_) => StatusCode::FORBIDDEN,
            Error::Sqlx(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Cache(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::CacheSerialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(body)).into_response()
    }
}
