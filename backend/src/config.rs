use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use std::fmt;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub sessions: SessionsConfig,
    pub jwt: JwtConfig,
    pub cookies: crate::services::cookies::CookieConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub user: String,
    #[serde(skip_serializing)]
    pub password: SecretString,
    pub host: String,
    pub port: u16,
    pub database: String,
}

impl Config {
    /// Load configuration from environment variables, with defaults.
    pub fn load() -> Result<Self, config::ConfigError> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            .add_source(config::Config::try_from(&Self::default())?)
            // Override with environment variables using `BUILDSCALE_` prefix and `_` separator
            // e.g., BUILDSCALE_DATABASE_USER="my_user"
            .add_source(
                config::Environment::with_prefix("BUILDSCALE")
                    .prefix_separator("_")
                    .separator("_"),
            )
            .build()?;

        config.try_deserialize()
    }
}

impl DatabaseConfig {
    /// Constructs the database connection string.
    pub fn connection_string(&self) -> SecretString {
        SecretString::from(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database
        ))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionsConfig {
    /// Session expiration time in hours (default: 720 hours = 30 days)
    /// Used for both initial session expiration AND maximum extension time
    pub expiration_hours: i64,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            expiration_hours: 720, // 30 days
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    /// Secret key for signing JWT access tokens (minimum 32 characters recommended)
    pub secret: String,
    /// Access token expiration time in minutes (default: 15 minutes)
    pub access_token_expiration_minutes: i64,
    /// Secret key for HMAC signing refresh tokens (minimum 32 characters recommended)
    pub refresh_token_secret: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "change-this-secret-in-production-min-32-chars".to_string(),
            access_token_expiration_minutes: 15,
            refresh_token_secret: "change-this-refresh-secret-in-production-min-32-chars".to_string(),
        }
    }
}

// Default values for the database configuration
impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            user: "postgres".to_string(),
            password: "password".to_string().into(),
            host: "localhost".to_string(),
            port: 5432,
            database: "postgres".to_string(),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use serde to serialize to pretty JSON
        // Password is automatically skipped due to #[serde(skip)]
        match serde_json::to_string_pretty(&self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "Error serializing config"),
        }
    }
}
