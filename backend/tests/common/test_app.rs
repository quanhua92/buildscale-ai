use axum::{Router, routing::get};
use backend::{load_config, Cache, CacheConfig, AppState};
use backend::handlers::health::health_check;
use reqwest::{Client, redirect::Policy};
use std::net::SocketAddr;
use tokio::net::TcpListener;

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
    pub config: backend::Config,
    /// Cache instance
    pub cache: Cache<String>,
}

impl TestApp {
    /// Create a new HTTP test app with server on random port
    ///
    /// # How it works:
    /// 1. Creates an Axum router with /api/v1 routes
    /// 2. Binds to port 0 (OS assigns random available port)
    /// 3. Starts server in background task
    /// 4. Creates reqwest client configured for testing
    /// 5. Waits 100ms for server to be ready
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
        // Load config
        let config = load_config().expect("Failed to load config");

        // Initialize cache
        let cache = Cache::new_local(CacheConfig {
            cleanup_interval_seconds: 60,
            default_ttl_seconds: Some(3600),
        });

        // Build application state
        let app_state = AppState::new(cache.clone());

        // Build API v1 routes
        let api_routes = Router::new()
            .route("/health", get(health_check));

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

        // Create HTTP client with persistent cookies
        let client = Client::builder()
            .redirect(Policy::none())
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            address,
            client,
            config,
            cache,
        }
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
