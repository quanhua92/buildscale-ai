use axum::Router;
use buildscale::{load_config, Cache, CacheConfig, AppState, DbPool, create_api_router};
use reqwest::{Client, redirect::Policy};
use secrecy::ExposeSecret;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use buildscale::models::users::User;

/// Configuration options for TestApp
///
/// This struct allows fine-grained control over TestApp behavior
/// and can be extended with new options in the future.
#[derive(Debug, Clone)]
pub struct TestAppOptions {
    /// Enable cookie storage for HTTP client
    /// - true: persists cookies across requests (for browser testing)
    /// - false: no cookie persistence (for API testing with Bearer tokens)
    pub cookie_store: bool,
}

impl Default for TestAppOptions {
    fn default() -> Self {
        Self {
            cookie_store: true,  // Default: enable cookies for backward compatibility
        }
    }
}

impl TestAppOptions {
    /// Create options for API testing (cookies disabled)
    pub fn api() -> Self {
        Self {
            cookie_store: false,
        }
    }

    /// Create options for browser testing (cookies enabled)
    pub fn browser() -> Self {
        Self {
            cookie_store: true,
        }
    }
}

/// HTTP test application wrapper
///
/// Manages an Axum server running on a random port for HTTP testing.
/// Each test gets its own server instance to allow parallel test execution.
pub struct TestApp {
    /// Server base URL (e.g., "http://127.0.0.1:54321")
    pub address: String,
    /// HTTP client for making requests
    pub client: Client,
    /// Application config
    pub config: buildscale::Config,
    /// Cache instance
    pub cache: Cache<String>,
    /// Database pool
    pub pool: DbPool,
}

impl TestApp {
    /// Create a new HTTP test app with default options
    ///
    /// This uses default options (cookies enabled) for backward compatibility.
    /// For API testing without cookies, use `TestApp::new_with_options(TestAppOptions::api())` instead.
    ///
    /// # Example
    /// ```rust
    /// #[tokio::test]
    /// async fn test_health_endpoint() {
    ///     let app = TestApp::new().await;
    ///
    ///     let response = app.client
    ///         .get(&format!("{}/api/v1/health", app.address))
    ///         .send()
    ///         .await
    ///         .unwrap();
    ///
    ///     assert_eq!(response.status(), 200);
    /// }
    /// ```
    pub async fn new() -> Self {
        Self::new_with_options(TestAppOptions::default()).await
    }

    /// Create a new HTTP test app with custom options
    ///
    /// # Parameters
    /// * `options` - Configuration options for the test app
    ///
    /// # Example
    /// ```rust
    /// // API testing without cookies
    /// let app = TestApp::new_with_options(TestAppOptions::api()).await;
    ///
    /// // Browser testing with cookies (also the default)
    /// let app = TestApp::new_with_options(TestAppOptions::browser()).await;
    ///
    /// // Custom options
    /// let app = TestApp::new_with_options(TestAppOptions { cookie_store: false }).await;
    /// ```
    pub async fn new_with_options(options: TestAppOptions) -> Self {
        // Load config
        let mut config = load_config().expect("Failed to load config");

        // Disable storage worker background processing in tests to avoid race conditions
        // during manual logic verification.
        config.storage_worker.cleanup_batch_size = 0;

        // Initialize cache
        let cache = Cache::new_local(CacheConfig {
            cleanup_interval_seconds: 60,
            default_ttl_seconds: Some(3600),
        });

        // Initialize user cache
        let user_cache: Cache<User> = Cache::new_local(CacheConfig::default());

        // Create database pool
        let pool = DbPool::connect(config.database.connection_string().expose_secret())
            .await
            .expect("Failed to connect to database");

        // Initialize Rig service for tests (dummy key)
        let rig_service = std::sync::Arc::new(buildscale::services::chat::rig_engine::RigService::dummy());

        // Initialize archive cleanup channel
        let (archive_cleanup_tx, _archive_cleanup_rx) = tokio::sync::mpsc::unbounded_channel();

        // Build application state with cache, user_cache, database pool, and config
        let app_state = AppState::new(cache.clone(), user_cache, pool.clone(), rig_service, config.clone(), archive_cleanup_tx);

        // Build API v1 routes using the shared router function
        let api_routes = create_api_router(app_state.clone());

        // Build the main router with nested API routes
        let app = Router::new()
            .nest("/api/v1", api_routes)
            .with_state(app_state);

        // Bind to random port (port 0 tells OS to assign available port)
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind random port");
        let port = listener.local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{port}");

        // Start server in background
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });

        // Give server time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Create HTTP client with configurable cookie storage
        let client = Client::builder()
            .redirect(Policy::none())
            .cookie_store(options.cookie_store)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            address,
            client,
            config,
            cache,
            pool,
        }
    }

    /// Get a database connection
    pub async fn get_connection(&self) -> sqlx::pool::PoolConnection<sqlx::Postgres> {
        self.pool.acquire().await.expect("Failed to acquire connection")
    }

    /// Get the full URL for an API endpoint
    ///
    /// # Example
    /// ```rust
    /// let url = app.url("/api/v1/health");
    /// // Returns: "http://127.0.0.1:54321/api/v1/health"
    /// ```
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.address, path)
    }
}
