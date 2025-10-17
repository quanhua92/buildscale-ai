pub mod config;
pub mod database;
pub mod error;
pub mod models;
pub mod queries;
pub mod services;
pub mod validation;

pub use config::Config;
pub use database::{DbConn, DbPool};

/// Load configuration from environment variables
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    Ok(Config::load()?)
}
