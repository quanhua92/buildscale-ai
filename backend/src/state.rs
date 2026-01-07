use crate::{cache::Cache, database::DbPool};

/// Application state shared across all HTTP handlers
///
/// This struct contains shared resources that need to be accessed
/// by API handlers, such as the cache instance and database pool.
#[derive(Clone)]
pub struct AppState {
    /// Cache instance for storing and retrieving data
    pub cache: Cache<String>,
    /// Database connection pool for accessing the database
    pub pool: DbPool,
}

impl AppState {
    /// Create a new AppState instance
    ///
    /// # Arguments
    /// * `cache` - Cache instance to use
    /// * `pool` - Database connection pool
    pub fn new(cache: Cache<String>, pool: DbPool) -> Self {
        Self { cache, pool }
    }
}
