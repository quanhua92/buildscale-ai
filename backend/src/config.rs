use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use std::fmt;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub sessions: SessionsConfig,
    pub jwt: JwtConfig,
    pub cache: CacheConfig,
    pub cookies: crate::services::cookies::CookieConfig,
    pub server: ServerConfig,
    pub ai: AiConfig,
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
            // Override with environment variables using `BUILDSCALE` prefix and `__` separator
            // e.g., BUILDSCALE__DATABASE__USER="my_user" or BUILDSCALE__JWT__REFRESH_TOKEN_SECRET
            .add_source(
                config::Environment::with_prefix("BUILDSCALE")
                    .prefix_separator("__")
                    .separator("__"), // Use double underscore consistently for prefix and nesting
            )
            .build()?;

        let config: Config = config.try_deserialize()?;

        // Validate JWT secrets
        config.validate().map_err(|e| {
            config::ConfigError::Message(format!("Configuration validation failed: {}", e))
        })?;

        Ok(config)
    }

    /// Validates JWT secrets meet security requirements
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate JWT access token secret
        let secret = self.jwt.secret.expose_secret();
        if secret.len() < 32 {
            return Err(format!(
                "BUILDSCALE__JWT__SECRET must be at least 32 characters (got {} chars). \
                 Set a strong secret in your .env file or environment.",
                secret.len()
            )
            .into());
        }

        // Validate JWT refresh token secret
        let refresh_secret = self.jwt.refresh_token_secret.expose_secret();
        if refresh_secret.len() < 32 {
            return Err(format!(
                "BUILDSCALE__JWT__REFRESH_TOKEN_SECRET must be at least 32 characters (got {} chars). \
                 Set a strong secret in your .env file or environment.",
                refresh_secret.len()
            ).into());
        }

        // Check for default/weak patterns in both secrets
        let weak_patterns = vec!["change-this", "secret", "password", "123456", "example"];

        for pattern in weak_patterns {
            if secret.to_lowercase().contains(pattern) {
                return Err(format!(
                    "BUILDSCALE__JWT__SECRET contains weak pattern '{}'. Use a cryptographically random secret.",
                    pattern
                ).into());
            }
            if refresh_secret.to_lowercase().contains(pattern) {
                return Err(format!(
                    "BUILDSCALE__JWT__REFRESH_TOKEN_SECRET contains weak pattern '{}'. Use a cryptographically random secret.",
                    pattern
                ).into());
            }
        }

        Ok(())
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

    /// How long to keep revoked tokens for theft detection (default: 1440 minutes = 1 day)
    /// Tokens older than this are automatically cleaned up by the background worker
    /// Recommended: 10 minutes (grace period + theft detection window) to 1 day
    pub revoked_token_retention_minutes: i64,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            expiration_hours: 720,                 // 30 days
            revoked_token_retention_minutes: 1440, // 1 day
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    /// User cache TTL in seconds (default: 900 = 15 minutes)
    /// Matches JWT access token expiration for consistency
    pub user_cache_ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            user_cache_ttl_seconds: 900, // 15 minutes
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    /// Secret key for signing JWT access tokens (minimum 32 characters recommended)
    #[serde(skip_serializing)]
    pub secret: SecretString,
    /// Access token expiration time in minutes (default: 15 minutes)
    #[serde(alias = "accessTokenExpirationMinutes")]
    pub access_token_expiration_minutes: i64,
    /// Secret key for HMAC signing refresh tokens (minimum 32 characters recommended)
    #[serde(skip_serializing)]
    #[serde(alias = "refreshTokenSecret")]
    #[serde(default = "JwtConfig::default_refresh_token_secret")]
    pub refresh_token_secret: SecretString,
}

// Custom Debug implementation to redact secrets
impl fmt::Debug for JwtConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwtConfig")
            .field("secret", &"<REDACTED>")
            .field(
                "access_token_expiration_minutes",
                &self.access_token_expiration_minutes,
            )
            .field("refresh_token_secret", &"<REDACTED>")
            .finish()
    }
}

impl JwtConfig {
    fn default_refresh_token_secret() -> SecretString {
        SecretString::from(String::new())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Host address to bind to (default: "0.0.0.0")
    pub host: String,
    /// Port to listen on (default: 3000)
    pub port: u16,
    /// Path to admin frontend build directory (default: "./admin")
    /// Set to "/app/admin" in Docker, "./admin" for local development
    /// Empty string disables admin frontend serving (security feature)
    pub admin_build_path: String,
    /// Path to web frontend build directory (default: "./web")
    /// Set to "/app/web" in Docker, "./web" for local development
    /// Empty string disables web frontend serving (security feature)
    pub web_build_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiConfig {
    /// Default window size for AI text chunking (in characters)
    pub chunk_window_size: usize,
    /// Default overlap for AI text chunking (in characters)
    pub chunk_overlap: usize,
    /// Dimension for AI embeddings
    pub embedding_dimension: usize,
    /// Default AI persona for chat sessions
    pub default_persona: String,
    /// Default context token limit for chat sessions
    pub default_context_token_limit: usize,
    /// Inactivity timeout for chat actors in seconds (default: 600)
    pub actor_inactivity_timeout_seconds: u64,
    /// OpenAI API key
    #[serde(skip_serializing)]
    pub openai_api_key: SecretString,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            chunk_window_size: 1000,
            chunk_overlap: 200,
            embedding_dimension: 1536,
            default_persona:
                "You are BuildScale AI, a professional software engineering assistant.".to_string(),
            default_context_token_limit: 4000,
            actor_inactivity_timeout_seconds: 600,
            openai_api_key: SecretString::from(String::new()),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            admin_build_path: "./admin".to_string(),
            web_build_path: "./web".to_string(),
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        // Require explicit configuration - no weak defaults
        Self {
            secret: SecretString::from(String::new()),
            access_token_expiration_minutes: 15,
            refresh_token_secret: SecretString::from(String::new()),
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
