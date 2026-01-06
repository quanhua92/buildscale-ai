use crate::cache::Cache;

/// Application state shared across all HTTP handlers
///
/// This struct contains shared resources that need to be accessed
/// by API handlers, such as the cache instance.
#[derive(Clone)]
pub struct AppState {
    /// Cache instance for storing and retrieving data
    pub cache: Cache<String>,
}

impl AppState {
    /// Create a new AppState instance
    ///
    /// # Arguments
    /// * `cache` - Cache instance to use
    pub fn new(cache: Cache<String>) -> Self {
        Self { cache }
    }
}
