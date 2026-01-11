use thiserror::Error;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// Import Axum types for HTTP response conversion
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

// Import tracing for error logging
use tracing as _;

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

/// Helper function to create standardized error response bodies
fn create_error_body(msg: String, code: &str) -> serde_json::Value {
    serde_json::json!({ "error": msg, "code": code })
}

/// Convert custom Error to HTTP response
///
/// This implementation maps each error variant to an appropriate HTTP status code
/// and returns a JSON response with an error message and error code.
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        // Log the error before returning response (safety net for all errors)
        // Uses appropriate log level based on error type
        match &self {
            Error::Validation(_) => {
                tracing::warn!(
                    error_code = "VALIDATION_ERROR",
                    error = %self,
                    status_code = 400,
                    "Error returned to client"
                );
            }
            Error::NotFound(_) => {
                tracing::warn!(
                    error_code = "NOT_FOUND",
                    error = %self,
                    status_code = 404,
                    "Error returned to client"
                );
            }
            Error::Forbidden(_) => {
                tracing::warn!(
                    error_code = "FORBIDDEN",
                    error = %self,
                    status_code = 403,
                    "Error returned to client"
                );
            }
            Error::Conflict(_) => {
                tracing::warn!(
                    error_code = "CONFLICT",
                    error = %self,
                    status_code = 409,
                    "Error returned to client"
                );
            }
            Error::Authentication(_) => {
                tracing::warn!(
                    error_code = "AUTHENTICATION_FAILED",
                    error = %self,
                    status_code = 401,
                    "Error returned to client"
                );
            }
            Error::InvalidToken(_) => {
                tracing::warn!(
                    error_code = "INVALID_TOKEN",
                    error = %self,
                    status_code = 401,
                    "Error returned to client"
                );
            }
            Error::SessionExpired(_) => {
                tracing::warn!(
                    error_code = "SESSION_EXPIRED",
                    error = %self,
                    status_code = 401,
                    "Error returned to client"
                );
            }
            Error::TokenTheftDetected(_) => {
                tracing::warn!(
                    error_code = "TOKEN_THEFT",
                    error = %self,
                    status_code = 403,
                    "Error returned to client"
                );
            }
            _ => {
                tracing::error!(
                    error_code = %self.error_code(),
                    error = %self,
                    status_code = %self.status_code(),
                    "Error returned to client"
                );
            }
        }

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
            Error::NotFound(msg) => (create_error_body(msg, "NOT_FOUND"), StatusCode::NOT_FOUND),
            Error::Forbidden(msg) => (create_error_body(msg, "FORBIDDEN"), StatusCode::FORBIDDEN),
            Error::Conflict(msg) => (create_error_body(msg, "CONFLICT"), StatusCode::CONFLICT),
            Error::Authentication(msg) => (create_error_body(msg, "AUTHENTICATION_FAILED"), StatusCode::UNAUTHORIZED),
            Error::InvalidToken(msg) => (create_error_body(msg, "INVALID_TOKEN"), StatusCode::UNAUTHORIZED),
            Error::SessionExpired(msg) => (create_error_body(msg, "SESSION_EXPIRED"), StatusCode::UNAUTHORIZED),
            Error::TokenTheftDetected(msg) => (create_error_body(msg, "TOKEN_THEFT"), StatusCode::FORBIDDEN),
            Error::Sqlx(_) => (create_error_body("Database error".to_string(), "INTERNAL_ERROR"), StatusCode::INTERNAL_SERVER_ERROR),
            Error::Internal(msg) => (create_error_body(msg, "INTERNAL_ERROR"), StatusCode::INTERNAL_SERVER_ERROR),
            Error::Config(_) => (create_error_body("Configuration error".to_string(), "CONFIG_ERROR"), StatusCode::INTERNAL_SERVER_ERROR),
            Error::Cache(msg) => (create_error_body(msg, "CACHE_ERROR"), StatusCode::INTERNAL_SERVER_ERROR),
            Error::CacheSerialization(msg) => (create_error_body(msg, "CACHE_ERROR"), StatusCode::INTERNAL_SERVER_ERROR),
            Error::Io(_) => (create_error_body("IO error".to_string(), "INTERNAL_ERROR"), StatusCode::INTERNAL_SERVER_ERROR),
        };

        (status, Json(body)).into_response()
    }
}

impl Error {
    /// Helper method to get the HTTP status code for an error
    fn status_code(&self) -> u16 {
        match self {
            Error::Validation(_) => 400,
            Error::NotFound(_) => 404,
            Error::Forbidden(_) => 403,
            Error::Conflict(_) => 409,
            Error::Authentication(_) | Error::InvalidToken(_) | Error::SessionExpired(_) => 401,
            Error::TokenTheftDetected(_) => 403,
            _ => 500,
        }
    }

    /// Helper method to get the error code for logging
    fn error_code(&self) -> &'static str {
        match self {
            Error::Validation(_) => "VALIDATION_ERROR",
            Error::NotFound(_) => "NOT_FOUND",
            Error::Forbidden(_) => "FORBIDDEN",
            Error::Conflict(_) => "CONFLICT",
            Error::Authentication(_) => "AUTHENTICATION_FAILED",
            Error::InvalidToken(_) => "INVALID_TOKEN",
            Error::SessionExpired(_) => "SESSION_EXPIRED",
            Error::TokenTheftDetected(_) => "TOKEN_THEFT",
            Error::Sqlx(_) => "INTERNAL_ERROR",
            Error::Internal(_) => "INTERNAL_ERROR",
            Error::Config(_) => "CONFIG_ERROR",
            Error::Cache(_) => "CACHE_ERROR",
            Error::CacheSerialization(_) => "CACHE_ERROR",
            Error::Io(_) => "INTERNAL_ERROR",
        }
    }
}
