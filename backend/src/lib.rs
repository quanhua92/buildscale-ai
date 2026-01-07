pub mod cache;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod models;
pub mod queries;
pub mod services;
pub mod state;
pub mod validation;

pub use cache::{Cache, CacheConfig, CacheHealthMetrics, run_cache_cleanup};
pub use config::Config;
pub use database::{DbConn, DbPool};
pub use handlers::health::health_check;
pub use state::AppState;

/// Load configuration from environment variables
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    Ok(Config::load()?)
}

use axum::{Router, routing::get};
use tokio::net::TcpListener;

/// Start the Axum API server
///
/// # Arguments
/// * `config` - Server configuration (host, port)
/// * `cache` - Cache instance to pass to handlers
///
/// # Returns
/// Returns Ok(()) when server shuts down, or Err on startup failure
///
/// # Example
/// ```no_run
/// use backend::{Config, Cache, CacheConfig, run_api_server};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = Config::load()?;
///     let cache: Cache<String> = Cache::new_local(CacheConfig::default());
///     run_api_server(&config, cache).await?;
///     Ok(())
/// }
/// ```
pub async fn run_api_server(
    config: &Config,
    cache: Cache<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build the application state
    let app_state = AppState::new(cache);

    // Build API v1 routes
    let api_routes = Router::new()
        .route("/health", get(health_check));

    // Build the main router with nested API routes
    let app = Router::new()
        .nest("/api/v1", api_routes)
        .with_state(app_state);

    // Bind to address
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;

    println!("API server listening on http://{}", addr);

    // Start server
    axum::serve(listener, app).await?;

    Ok(())
}
