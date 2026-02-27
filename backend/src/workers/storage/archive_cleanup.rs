use crate::fs::storage::FileStorageService;
use crate::state::ArchiveCleanupMessage;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{info, warn};

/// Background worker that handles immediate archive blob cleanup
///
/// Listens for cleanup messages and deletes physical blobs immediately.
/// In the simplified schema, we don't use a queue table - cleanup is done
/// immediately when files are purged.
pub async fn archive_cleanup_worker(
    _pool: sqlx::PgPool,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    mut archive_cleanup_rx: mpsc::UnboundedReceiver<ArchiveCleanupMessage>,
    worker_config: crate::config::StorageWorkerConfig,
    storage_config: crate::config::StorageConfig,
) {
    // Initialize once to avoid redundant I/O and allocations in the loop
    let storage = FileStorageService::new(&storage_config.base_path);
    let _cleanup_interval = interval(Duration::from_secs(worker_config.cleanup_interval_seconds));

    info!("[StorageWorker] Started (listening for cleanup messages)");

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("[StorageWorker] Shutting down");
                break;
            }
            message = archive_cleanup_rx.recv() => {
                if let Some(msg) = message {
                    // Process message hashes immediately
                    for hash in msg.hashes {
                        // Delete the physical blob
                        if let Err(e) = storage.delete_archive_blob(msg.workspace_id, &hash).await {
                            warn!("[StorageWorker] Failed to delete blob {}: {}", hash, e);
                        } else {
                            info!("[StorageWorker] Deleted orphaned blob {}", hash);
                        }
                    }
                }
            }
        }
    }

    info!("[StorageWorker] Stopped");
}
