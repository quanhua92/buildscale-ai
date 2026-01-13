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
pub use error::{Error, Result, ValidationErrors};
pub use handlers::{auth::login, auth::logout, auth::me, auth::register, auth::refresh, health::health_check, health::health_cache};
pub use middleware::auth::AuthenticatedUser;
pub use state::AppState;
pub use workers::revoked_token_cleanup_worker;

/// Load configuration from environment variables
pub fn load_config() -> Result<Config> {
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

use axum::{Router, routing::{get, post, patch, delete}, middleware as axum_middleware, response::Response, extract::Request, http::HeaderName};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::compression::CompressionLayer;
use std::path::Path;
use axum::middleware::Next;
use uuid::Uuid;
use crate::middleware::auth::jwt_auth_middleware;

/// Middleware to add request ID to response headers
async fn request_id_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let id = Uuid::now_v7().to_string();
            req.headers_mut().insert(
                HeaderName::from_static("x-request-id"),
                id.parse().unwrap()
            );
            id
        });

    let mut response = next.run(req).await;
    response.headers_mut().insert(
        HeaderName::from_static("x-request-id"),
        request_id.parse().unwrap(),
    );

    response
}

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
                .route("/auth/me", get(me))
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_middleware,
                ))
        )
        // Add workspace routes with their own security middleware
        .nest("/workspaces", create_workspace_router(state.clone()))
}

/// Create workspace routes with access control middleware
///
/// This router has a split architecture:
/// - Collection routes (/workspaces) use JWT auth only
/// - Item routes (/workspaces/:id) use JWT + workspace access middleware
///
/// # Security Model
/// - POST /workspaces: Any authenticated user can create
/// - GET /workspaces: Returns only user's workspaces (owner OR member)
/// - GET /workspaces/:id: Requires workspace membership
/// - PATCH /workspaces/:id: Requires workspace ownership
/// - DELETE /workspaces/:id: Requires workspace ownership
///
/// # Arguments
/// * `state` - Application state containing cache, user_cache, and database pool
///
/// # Returns
/// A configured Router with workspace routes
fn create_workspace_router(state: AppState) -> Router<AppState> {
    use crate::handlers::workspaces as workspace_handlers;

    Router::new()
        // Collection routes (JWT auth only, no workspace membership needed)
        .merge(
            Router::new()
                .route("/", post(workspace_handlers::create_workspace))
                .route("/", get(workspace_handlers::list_workspaces))
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_middleware,
                ))
        )
        // Item routes (JWT auth + workspace membership required)
        .merge(
            Router::new()
                .route("/{id}", get(workspace_handlers::get_workspace))
                .route("/{id}", patch(workspace_handlers::update_workspace))
                .route("/{id}", delete(workspace_handlers::delete_workspace))
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    crate::middleware::workspace_access::workspace_access_middleware,
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
) -> Result<()> {
    use secrecy::ExposeSecret;

    // Create database connection pool
    let pool = DbPool::connect(config.database.connection_string().expose_secret())
        .await
        .map_err(|e| Error::Internal(format!("Failed to connect to database: {}", e)))?;

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

    let api_routes = create_api_router(app_state.clone());

    // Start with API routes
    let mut app = Router::new()
        .nest("/api/v1", api_routes);

    // Add admin frontend at /admin (only if path is configured and not empty)
    let admin_build_path = &config.server.admin_build_path;
    if !admin_build_path.is_empty() {
        tracing::info!("Admin frontend serving enabled at path: '{}'", admin_build_path);

        if !Path::new(admin_build_path).is_dir() {
            tracing::warn!(
                "Admin build directory not found at '{}'. Admin frontend will fail to serve.",
                admin_build_path
            );
        }

        let admin_index_path = Path::new(admin_build_path).join("index.html");
        let admin_static_service = ServeDir::new(admin_build_path)
            .not_found_service(ServeFile::new(admin_index_path));

        app = app.nest_service("/admin", admin_static_service);
    } else {
        tracing::info!("Admin frontend serving disabled (admin_build_path is empty)");
    }

    // Add web frontend fallback at root / (only if path is configured and not empty)
    let web_build_path = &config.server.web_build_path;
    if !web_build_path.is_empty() {
        tracing::info!("Web frontend serving enabled at path: '{}'", web_build_path);

        if !Path::new(web_build_path).is_dir() {
            tracing::warn!(
                "Web build directory not found at '{}'. Web frontend will fail to serve.",
                web_build_path
            );
        }

        let web_index_path = Path::new(web_build_path).join("index.html");
        let web_static_service = ServeDir::new(web_build_path)
            .not_found_service(ServeFile::new(web_index_path));

        app = app.fallback_service(web_static_service);
    } else {
        tracing::info!("Web frontend serving disabled (web_build_path is empty)");
    }

    // Apply middleware layers to the combined app
    let app = app.layer(
        ServiceBuilder::new()
            .layer(axum_middleware::from_fn(request_id_middleware))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|request: &Request<_>| {
                        let request_id = request
                            .headers()
                            .get("x-request-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("unknown");

                        tracing::info_span!(
                            "http_request",
                            method = %request.method(),
                            path = %request.uri().path(),
                            request_id = %request_id,
                            status = tracing::field::Empty,
                            latency = tracing::field::Empty,
                        )
                    })
                    .on_request(
                        tower_http::trace::DefaultOnRequest::new()
                            .level(tracing::Level::DEBUG)
                    )
                    .on_response(
                        tower_http::trace::DefaultOnResponse::new()
                            .level(tracing::Level::DEBUG)
                    ),
            )
            .layer(
                SetResponseHeaderLayer::if_not_present(
                    axum::http::header::X_CONTENT_TYPE_OPTIONS,
                    axum::http::HeaderValue::from_static("nosniff"),
                ),
            )
            .layer(
                SetResponseHeaderLayer::if_not_present(
                    axum::http::header::X_FRAME_OPTIONS,
                    axum::http::HeaderValue::from_static("DENY"),
                ),
            )
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
            .layer(CompressionLayer::new()),
    )
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
