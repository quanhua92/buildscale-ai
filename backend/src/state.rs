use crate::{
    cache::Cache, config::Config, database::DbPool, models::users::User,
    services::chat::registry::AgentRegistry, services::chat::rig_engine::RigService,
    services::storage::FileStorageService,
};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Message sent to the archive cleanup worker
#[derive(Debug, Clone)]
pub struct ArchiveCleanupMessage {
    pub workspace_id: uuid::Uuid,
    pub hashes: Vec<String>,
}

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
    /// File storage service (Disk I/O)
    pub storage: Arc<FileStorageService>,
    /// Application configuration
    pub config: Config,
    /// Channel to notify archive cleanup worker
    pub archive_cleanup_tx: mpsc::UnboundedSender<ArchiveCleanupMessage>,
}

impl AppState {
    /// Create a new AppState instance
    ///
    /// # Arguments
    /// * `cache` - Cache instance to use
    /// * `user_cache` - User cache instance to use
    /// * `pool` - Database connection pool
    /// * `rig_service` - Rig service instance
    /// * `config` - Application configuration
    /// * `archive_cleanup_tx` - Channel sender for archive cleanup
    pub fn new(
        cache: Cache<String>,
        user_cache: Cache<User>,
        pool: DbPool,
        rig_service: Arc<RigService>,
        config: Config,
        archive_cleanup_tx: mpsc::UnboundedSender<ArchiveCleanupMessage>,
    ) -> Self {
        let storage = Arc::new(FileStorageService::new(&config.storage.base_path));
        // Note: Storage init is async, so we might want to call it from main before creating AppState,
        // or just let it create directories lazily/on-startup.
        // For this implementation, we assume main.rs might call init, or we lazily handle it.

        Self {
            cache,
            user_cache,
            pool,
            agents: Arc::new(AgentRegistry::new()),
            rig_service,
            storage,
            config,
            archive_cleanup_tx,
        }
    }
}
