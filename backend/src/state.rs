use crate::{
    cache::Cache, database::DbPool, models::users::User, services::chat::registry::AgentRegistry,
    services::chat::rig_engine::RigService,
};
use std::sync::Arc;

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
    /// Registry for active AI agents
    pub agents: Arc<AgentRegistry>,
    /// Service for interacting with Rig.rs AI runtime
    pub rig_service: Arc<RigService>,
}

impl AppState {
    /// Create a new AppState instance
    ///
    /// # Arguments
    /// * `cache` - Cache instance to use
    /// * `user_cache` - User cache instance to use
    /// * `pool` - Database connection pool
    /// * `rig_service` - Rig service instance
    pub fn new(
        cache: Cache<String>,
        user_cache: Cache<User>,
        pool: DbPool,
        rig_service: Arc<RigService>,
    ) -> Self {
        Self {
            cache,
            user_cache,
            pool,
            agents: Arc::new(AgentRegistry::new()),
            rig_service,
        }
    }
}
