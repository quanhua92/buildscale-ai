use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use std::fmt;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    pub database: DatabaseConfig,
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
            // Override with environment variables using `BUILDSCALE__` prefix and `__` separator
            // e.g., BUILDSCALE__DATABASE__USER="my_user"
            .add_source(
                config::Environment::with_prefix("BUILDSCALE")
                    .prefix_separator("__")
                    .separator("__"),
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
