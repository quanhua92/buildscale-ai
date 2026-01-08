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
pub mod workers;

pub use cache::{Cache, CacheConfig, CacheHealthMetrics, run_cache_cleanup};
pub use config::Config;
pub use database::{DbConn, DbPool};
pub use handlers::{auth::login, auth::logout, auth::register, auth::refresh, health::health_check};
pub use state::AppState;
pub use workers::revoked_token_cleanup_worker;

/// Load configuration from environment variables
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    Ok(Config::load()?)
}

/// Initialize tracing subscriber with environment filter
///
/// This function sets up the tracing subscriber for the application.
/// It reads the RUST_LOG environment variable to set the log level.
/// If RUST_LOG is not set, it defaults to "info" level.
///
/// # Example
/// ```
/// use buildscale::init_tracing;
///
/// fn main() {
///     init_tracing();
/// }
/// ```
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_target(false)
        .init();
}

use axum::{Router, routing::{get, post}};
use tokio::net::TcpListener;

/// Create API v1 routes
///
/// This function creates the API router with all endpoints.
/// It's reused by both the main server and test apps to ensure consistency.
///
/// # Returns
/// A configured Router with all API v1 routes
pub fn create_api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
}

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
/// use buildscale::{Config, Cache, CacheConfig, run_api_server};
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
    use secrecy::ExposeSecret;

    // Create database connection pool
    let pool = DbPool::connect(config.database.connection_string().expose_secret())
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    // Spawn revoked token cleanup worker
    let (revoked_cleanup_shutdown_tx, _) = tokio::sync::broadcast::channel(1);
    let pool_clone = pool.clone();
    let revoked_cleanup_shutdown_tx_clone = revoked_cleanup_shutdown_tx.clone();

    tokio::spawn(async move {
        revoked_token_cleanup_worker(
            pool_clone,
            revoked_cleanup_shutdown_tx_clone.subscribe(),
        ).await;
    });

    // Build the application state with cache AND database pool
    let app_state = AppState::new(cache, pool);

    // Build API v1 routes using the shared router function
    let api_routes = create_api_router();

    // Build the main router with nested API routes
    let app = Router::new()
        .nest("/api/v1", api_routes)
        .with_state(app_state);

    // Bind to address
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("API server listening on http://{}", addr);

    // Setup shutdown handler
    let shutdown_signal = async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
        tracing::info!("Shutdown signal received");
        revoked_cleanup_shutdown_tx.send(()).ok();
    };

    // Start server with shutdown signal
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    Ok(())
}
