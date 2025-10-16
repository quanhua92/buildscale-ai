use thiserror::Error;

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
}

/// A type alias for `Result<T, Error>` to simplify function signatures.
pub type Result<T> = std::result::Result<T, Error>;
