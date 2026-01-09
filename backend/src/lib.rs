pub mod cache;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod queries;
pub mod services;
pub mod state;
pub mod validation;
pub mod workers;

pub use cache::{Cache, CacheConfig, CacheHealthMetrics, run_cache_cleanup};
pub use config::Config;
pub use database::{DbConn, DbPool};
pub use handlers::{auth::login, auth::logout, auth::register, auth::refresh, health::health_check, health::health_cache};
pub use middleware::auth::AuthenticatedUser;
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

/// Get the current git commit hash
///
/// Returns the commit hash from the GIT_COMMIT environment variable if set
/// (e.g., in Docker builds), or falls back to running git command.
fn get_git_commit_hash() -> String {
    // Check environment variable first (set in Docker builds)
    if let Ok(commit) = std::env::var("GIT_COMMIT") {
        if !commit.is_empty() {
            return commit;
        }
    }

    // Fallback: try to get the short commit hash from git
    use std::process::Command;
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            if let Ok(hash) = String::from_utf8(output.stdout) {
                return hash.trim().to_string();
            }
        }
    }

    // Final fallback if git is not available or not in a git repo
    "unknown".to_string()
}

/// Get the build timestamp
///
/// Returns the build date from the BUILD_DATE environment variable if set
/// (e.g., in Docker builds), or "unknown".
fn get_build_date() -> String {
    if let Ok(date) = std::env::var("BUILD_DATE") {
        if !date.is_empty() {
            return date;
        }
    }
    "unknown".to_string()
}

use axum::{Router, routing::{get, post}, middleware as axum_middleware};
use tokio::net::TcpListener;
use crate::middleware::auth::jwt_auth_middleware;

/// Create API v1 routes
///
/// This function creates the API router with all endpoints.
/// It's reused by both the main server and test apps to ensure consistency.
///
/// # Arguments
/// * `state` - Application state containing cache, user_cache, and database pool
///
/// # Returns
/// A configured Router with all API v1 routes
pub fn create_api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
        .merge(
            Router::new()
                .route("/health/cache", get(health_cache))
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_middleware,
                ))
        )
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

    // Create user cache with configured TTL
    let user_cache = Cache::new_local(CacheConfig::default());

    // Build the application state with cache, user_cache, and database pool
    let app_state = AppState::new(cache, user_cache, pool);

    // Build API v1 routes using the shared router function
    let api_routes = create_api_router(app_state.clone());

    // Build the main router with nested API routes
    let app = Router::new()
        .nest("/api/v1", api_routes)
        .with_state(app_state);

    // Bind to address
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;

    // Log server startup with build information
    let commit_hash = get_git_commit_hash();
    let build_date = get_build_date();
    tracing::info!(
        "API server listening on http://{} (commit: {}, built: {})",
        addr, commit_hash, build_date
    );

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
