use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

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

    /// A JSON serialization error.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// An LLM error.
    #[error("LLM error: {0}")]
    Llm(String),

    /// AI provider error.
    #[error("AI provider error: {0}")]
    AiProvider(String),

    /// Provider not configured.
    #[error("Provider '{0}' not configured")]
    ProviderNotConfigured(String),

    /// Invalid model format.
    #[error("Invalid model format: {0}")]
    InvalidModelFormat(String),

    /// Model not supported by provider.
    #[error("Model '{0}' not supported by provider '{1}'")]
    ModelNotSupported(String, String),

    /// API key missing for provider.
    #[error("API key not configured for provider '{0}'")]
    ApiKeyMissing(String),

    /// Model disabled.
    #[error("Model '{0}' is disabled")]
    ModelDisabled(String),
}

/// A type alias for `Result<T, Error>` to simplify function signatures.
pub type Result<T> = std::result::Result<T, Error>;

/// Helper function to create standardized error response bodies
fn create_error_body(msg: String, code: &str) -> serde_json::Value {
    serde_json::json!({ "error": msg, "code": code })
}

/// Log error at appropriate level based on error type
/// Client-facing errors (4xx) are logged as warnings, server errors (5xx) as errors
fn log_error(error: &Error, error_code: &str, status_code: u16) {
    if status_code >= 500 {
        tracing::error!(
            error_code,
            error = %error,
            status_code,
            "Error returned to client"
        );
    } else {
        tracing::warn!(
            error_code,
            error = %error,
            status_code,
            "Error returned to client"
        );
    }
}

/// Convert custom Error to HTTP response
///
/// This implementation maps each error variant to an appropriate HTTP status code
/// and returns a JSON response with an error message and error code.
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        // Log the error before returning response using helper
        log_error(&self, self.error_code(), self.status_code());

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
            Error::Authentication(msg) => (
                create_error_body(msg, "AUTHENTICATION_FAILED"),
                StatusCode::UNAUTHORIZED,
            ),
            Error::InvalidToken(msg) => (
                create_error_body(msg, "INVALID_TOKEN"),
                StatusCode::UNAUTHORIZED,
            ),
            Error::SessionExpired(msg) => (
                create_error_body(msg, "SESSION_EXPIRED"),
                StatusCode::UNAUTHORIZED,
            ),
            Error::TokenTheftDetected(msg) => {
                (create_error_body(msg, "TOKEN_THEFT"), StatusCode::FORBIDDEN)
            }
            Error::Sqlx(_) => (
                create_error_body("Database error".to_string(), "INTERNAL_ERROR"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::Internal(msg) => (
                create_error_body(msg, "INTERNAL_ERROR"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::Config(_) => (
                create_error_body("Configuration error".to_string(), "CONFIG_ERROR"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::Cache(msg) => (
                create_error_body(msg, "CACHE_ERROR"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::CacheSerialization(msg) => (
                create_error_body(msg, "CACHE_ERROR"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::Io(_) => (
                create_error_body("IO error".to_string(), "INTERNAL_ERROR"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::Json(e) => (
                create_error_body(format!("Invalid JSON payload: {}", e), "VALIDATION_ERROR"),
                StatusCode::BAD_REQUEST,
            ),
            Error::Llm(msg) => (
                create_error_body(msg, "LLM_ERROR"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::AiProvider(msg) => (
                create_error_body(msg, "AI_PROVIDER_ERROR"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::ProviderNotConfigured(provider) => (
                create_error_body(format!("Provider '{}' not configured", provider), "PROVIDER_NOT_CONFIGURED"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::InvalidModelFormat(msg) => (
                create_error_body(msg, "INVALID_MODEL_FORMAT"),
                StatusCode::BAD_REQUEST,
            ),
            Error::ModelNotSupported(model, provider) => (
                create_error_body(format!("Model '{}' not supported by provider '{}'", model, provider), "MODEL_NOT_SUPPORTED"),
                StatusCode::BAD_REQUEST,
            ),
            Error::ApiKeyMissing(provider) => (
                create_error_body(format!("API key not configured for provider '{}'", provider), "API_KEY_MISSING"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Error::ModelDisabled(model) => (
                create_error_body(format!("Model '{}' is disabled", model), "MODEL_DISABLED"),
                StatusCode::FORBIDDEN,
            ),
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
            Error::Json(_) => 400,
            Error::InvalidModelFormat(_) => 400,
            Error::ModelNotSupported(_, _) => 400,
            Error::ModelDisabled(_) => 403,
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
            Error::Json(_) => "JSON_ERROR",
            Error::Llm(_) => "LLM_ERROR",
            Error::AiProvider(_) => "AI_PROVIDER_ERROR",
            Error::ProviderNotConfigured(_) => "PROVIDER_NOT_CONFIGURED",
            Error::InvalidModelFormat(_) => "INVALID_MODEL_FORMAT",
            Error::ModelNotSupported(_, _) => "MODEL_NOT_SUPPORTED",
            Error::ApiKeyMissing(_) => "API_KEY_MISSING",
            Error::ModelDisabled(_) => "MODEL_DISABLED",
        }
    }
}
