use crate::{cache::Cache, database::DbPool, models::users::User};

/// Application state shared across all HTTP handlers
///
/// This struct contains shared resources that need to be accessed
/// by API handlers, such as the cache instance and database pool.
#[derive(Clone)]
pub struct AppState {
    /// Cache instance for storing and retrieving data
    pub cache: Cache<String>,
    /// User cache instance for storing authenticated user data
    pub user_cache: Cache<User>,
    /// Database connection pool for accessing the database
    pub pool: DbPool,
}

impl AppState {
    /// Create a new AppState instance
    ///
    /// # Arguments
    /// * `cache` - Cache instance to use
    /// * `user_cache` - User cache instance to use
    /// * `pool` - Database connection pool
    pub fn new(cache: Cache<String>, user_cache: Cache<User>, pool: DbPool) -> Self {
        Self { cache, user_cache, pool }
    }
}
