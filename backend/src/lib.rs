pub mod agent;
pub mod ai;
pub mod auth;
pub mod cache;
pub mod chat;
pub mod users;
pub mod workspaces;
pub mod config;
pub mod database;
pub mod error;
pub mod fs;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod providers;
pub mod state;
pub mod tools;
pub mod utils;
pub mod validation;
pub mod workers;

// Re-export commonly used types
pub use agent::{get_persona, AgentSession, AgentType, SessionStatus};
pub use cache::{Cache, CacheConfig, CacheHealthMetrics, run_cache_cleanup};
pub use config::Config;
pub use database::{DbConn, DbPool};
pub use error::{Error, Result, ValidationErrors};
pub use handlers::{
    list_workspace_sessions, get_session, pause_session, resume_session, cancel_session,
    login, logout, me, register, refresh,
    health_check, health_cache,
    list_members, get_my_membership, add_member, update_member_role, remove_member,
    create_workspace, list_workspaces, get_workspace, update_workspace, delete_workspace,
    create_file, get_file, create_version, update_file, delete_file, restore_file, purge_file, list_trash,
    add_tag, remove_tag, list_files_by_tag, create_link, remove_link, get_file_network,
    text_search,
    execute_tool,
    create_chat, get_chat, post_chat_message, stop_chat_generation, update_chat, get_chat_context, get_chat_events,
    list_chats,
    get_providers, get_workspace_providers,
};
pub use middleware::auth::AuthenticatedUser;
pub use state::AppState;
pub use workers::{revoked_token_cleanup_worker, archive_cleanup_worker};
pub use fs::workers::{tag_indexer_worker, link_indexer_worker};

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
/// init_tracing();
/// ```
pub fn init_tracing() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let mut final_filter = filter;

    // Always set external libraries to warn
    if !final_filter.contains("rig=") {
        final_filter = format!("{},rig=warn", final_filter);
    }
    if !final_filter.contains("rig_core=") {
        final_filter = format!("{},rig_core=warn", final_filter);
    }
    if !final_filter.contains("openai=") {
        final_filter = format!("{},openai=warn", final_filter);
    }

    // Set our modules to debug by default for better visibility
    if !final_filter.contains("buildscale::handlers::chat=") {
        final_filter = format!("{},buildscale::handlers::chat=debug", final_filter);
    }
    if !final_filter.contains("buildscale::chat::services::actor=") {
        final_filter = format!("{},buildscale::chat::services::actor=debug", final_filter);
    }
    if !final_filter.contains("buildscale::chat::services::registry=") {
        final_filter = format!("{},buildscale::chat::services::registry=debug", final_filter);
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(final_filter))
        .with_target(false)
        .init();
}

/// Get the current git commit hash
///
/// Returns the commit hash from the GIT_COMMIT environment variable if set
/// (e.g., in Docker builds), or falls back to running git command.
fn get_git_commit_hash() -> String {
    // Check environment variable first (set in Docker builds)
    if let Some(commit) = std::env::var("GIT_COMMIT").ok().filter(|c| !c.is_empty()) {
        return commit;
    }

    // Fallback: try to get the short commit hash from git
    use std::process::Command;
    if let Some(hash) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
    {
        return hash.trim().to_string();
    }

    // Final fallback if git is not available or not in a git repo
    "unknown".to_string()
}

/// Get the build timestamp
///
/// Returns the build date from the BUILD_DATE environment variable if set
/// (e.g., in Docker builds), or "unknown".
fn get_build_date() -> String {
    if let Some(date) = std::env::var("BUILD_DATE").ok().filter(|d| !d.is_empty()) {
        return date;
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
                .route("/providers", get(get_providers))
                // Agent session routes - global (scoped by session ownership)
                .route("/agent-sessions/{id}", get(crate::handlers::get_session))
                .route("/agent-sessions/{id}/pause", post(crate::handlers::pause_session))
                .route("/agent-sessions/{id}/resume", post(crate::handlers::resume_session))
                .route("/agent-sessions/{id}", delete(crate::handlers::cancel_session))
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_middleware,
                ))
        )
        // Add workspace routes with their own security middleware
        .nest("/workspaces", create_workspace_router(state.clone()))
}

/// Create workspace routes with JWT authentication
///
/// # Security Model
/// - POST /workspaces: Any authenticated user can create
/// - GET /workspaces: Returns only user's workspaces (owner OR member)
/// - GET /workspaces/:id: Requires workspace membership (validated by middleware)
/// - PATCH /workspaces/:id: Requires workspace ownership (validated by middleware)
/// - DELETE /workspaces/:id: Requires workspace ownership (validated by middleware)
///
/// # Arguments
/// * `state` - Application state containing cache, user_cache, and database pool
///
/// # Returns
/// A configured Router with workspace routes
fn create_workspace_router(state: AppState) -> Router<AppState> {
    use crate::handlers::{
        create_workspace, list_workspaces, get_workspace, update_workspace, delete_workspace,
        list_members, get_my_membership, add_member, update_member_role, remove_member,
        create_file, get_file, create_version, update_file, delete_file, restore_file, purge_file, list_trash,
        add_tag, remove_tag, list_files_by_tag, create_link, remove_link, get_file_network,
        text_search,
        execute_tool,
        create_chat, get_chat, post_chat_message, stop_chat_generation, update_chat, get_chat_context, get_chat_events,
        list_chats,
        get_workspace_providers,
        list_workspace_sessions,
    };
    use crate::middleware::workspace_access::workspace_access_middleware;

    Router::new()
        .route("/", post(create_workspace))
        .route("/", get(list_workspaces))
        .route(
            "/{id}",
            get(get_workspace)
                .patch(update_workspace)
                .delete(delete_workspace)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/members",
            get(list_members)
                .post(add_member)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/members/me",
            get(get_my_membership)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/members/{user_id}",
            patch(update_member_role)
                .delete(remove_member)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        // File routes
        .route(
            "/{id}/files",
            post(create_file)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/{file_id}",
            get(get_file)
                .patch(update_file)
                .delete(delete_file)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/{file_id}/restore",
            post(restore_file)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/{file_id}/purge",
            delete(purge_file)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/trash",
            get(list_trash)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/search",
            post(text_search)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/tags/{tag}",
            get(list_files_by_tag)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/{file_id}/tags",
            post(add_tag)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/{file_id}/tags/{tag}",
            delete(remove_tag)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/{file_id}/links",
            post(create_link)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/{file_id}/links/{target_id}",
            delete(remove_link)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/{file_id}/network",
            get(get_file_network)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/files/{file_id}/versions",
            post(create_version)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/tools",
            post(execute_tool)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        // Provider routes
        .route(
            "/{id}/providers",
            get(get_workspace_providers)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        // Chat routes
        .route(
            "/{id}/chats",
            get(list_chats)
                .post(create_chat)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/chats/{chat_id}/stop",
            post(stop_chat_generation)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/chats/{chat_id}",
            post(post_chat_message)
                .get(get_chat)
                .patch(update_chat)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/chats/{chat_id}/events",
            get(get_chat_events)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route(
            "/{id}/chats/{chat_id}/context",
            get(get_chat_context)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        // Agent session routes - workspace scoped
        .route(
            "/{id}/agent-sessions",
            get(list_workspace_sessions)
                .route_layer(axum_middleware::from_fn_with_state(
                    state.clone(),
                    workspace_access_middleware,
                )),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        ))
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
/// use buildscale::{Config, Cache, CacheConfig, run_api_server, chat::services::RigService};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = Config::load()?;
///     let cache: Cache<String> = Cache::new_local(CacheConfig::default());
///     let rig_service = Arc::new(RigService::from_env());
///     run_api_server(&config, cache, rig_service).await?;
///     Ok(())
/// }
/// ```
pub async fn run_api_server(
    config: &Config,
    cache: Cache<String>,
    rig_service: std::sync::Arc<crate::chat::services::RigService>,
) -> Result<()> {
    use secrecy::ExposeSecret;

    // Create database connection pool
    let pool = DbPool::connect(config.database.connection_string().expose_secret())
        .await
        .map_err(|e| Error::Internal(format!("Failed to connect to database: {}", e)))?;

    // Spawn cleanup workers
    let (cleanup_shutdown_tx, _) = tokio::sync::broadcast::channel(1);

    // Archive cleanup channel
    let (archive_cleanup_tx, archive_cleanup_rx) = tokio::sync::mpsc::unbounded_channel();

    // Tag indexer channel
    let (tag_index_tx, tag_index_rx) = tokio::sync::mpsc::unbounded_channel();

    // Link indexer channel
    let (link_index_tx, link_index_rx) = tokio::sync::mpsc::unbounded_channel();

    // Auth Worker
    let pool_auth = pool.clone();
    let shutdown_auth = cleanup_shutdown_tx.subscribe();
    tokio::spawn(async move {
        revoked_token_cleanup_worker(pool_auth, shutdown_auth).await;
    });

    // Storage Worker
    let pool_storage = pool.clone();
    let shutdown_storage = cleanup_shutdown_tx.subscribe();
    let worker_config = config.storage_worker.clone();
    let storage_config = config.storage.clone();
    tokio::spawn(async move {
        archive_cleanup_worker(pool_storage, shutdown_storage, archive_cleanup_rx, worker_config, storage_config).await;
    });

    // Tag Indexer Worker
    let pool_tags = pool.clone();
    let shutdown_tags = cleanup_shutdown_tx.subscribe();
    let storage_config_tags = config.storage.clone();
    tokio::spawn(async move {
        tag_indexer_worker(pool_tags, shutdown_tags, tag_index_rx, storage_config_tags).await;
    });

    // Link Indexer Worker
    let pool_links = pool.clone();
    let shutdown_links = cleanup_shutdown_tx.subscribe();
    let storage_config_links = config.storage.clone();
    tokio::spawn(async move {
        link_indexer_worker(pool_links, shutdown_links, link_index_rx, storage_config_links).await;
    });

    // Create user cache with configured TTL
    let user_cache = Cache::new_local(CacheConfig::default());

    // Build the application state with cache, user_cache, database pool, and config
    let app_state = AppState::new(cache, user_cache, pool, rig_service, config.clone(), archive_cleanup_tx, tag_index_tx, link_index_tx);

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
        cleanup_shutdown_tx.send(()).ok();
    };

    // Start server with shutdown signal
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    Ok(())
}
