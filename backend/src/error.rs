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
}

/// A type alias for `Result<T, Error>` to simplify function signatures.
pub type Result<T> = std::result::Result<T, Error>;
