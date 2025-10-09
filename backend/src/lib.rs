pub mod config;
pub mod models;

pub use config::Config;

/// Load configuration from environment variables
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    Ok(Config::load()?)
}
