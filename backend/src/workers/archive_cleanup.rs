use crate::queries::files;
use crate::services::storage::FileStorageService;
use crate::state::ArchiveCleanupMessage;
use std::time::Duration;
use tokio::time::interval;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use uuid::Uuid;
use sqlx::Connection;

/// Background worker that periodically cleans up orphaned archive blobs
///
/// Runs every hour to check the cleanup queue for stale hashes
pub async fn archive_cleanup_worker(
    pool: sqlx::PgPool,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    mut archive_cleanup_rx: mpsc::UnboundedReceiver<ArchiveCleanupMessage>,
    worker_config: crate::config::StorageWorkerConfig,
    storage_config: crate::config::StorageConfig,
) {
    // Initialize once to avoid redundant I/O and allocations in the loop
    let storage = FileStorageService::new(&storage_config.base_path);
    let mut cleanup_interval = interval(Duration::from_secs(worker_config.cleanup_interval_seconds));
    let batch_size = worker_config.cleanup_batch_size;

    info!("[StorageWorker] Started (runs every {}s or on-demand)", worker_config.cleanup_interval_seconds);

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("[StorageWorker] Shutting down");
                break;
            }
            message = archive_cleanup_rx.recv() => {
                if let Some(msg) = message {
                    // Optimized: Process message hashes immediately
                    for hash in msg.hashes {
                        // Since hashes are salted with version_id, they are globally unique.
                        // We can safely delete the physical blob immediately.
                        if let Err(e) = storage.delete_archive_blob(msg.workspace_id, &hash).await {
                            warn!("[StorageWorker] Failed to delete blob {}: {}", hash, e);
                        } else {
                            info!("[StorageWorker] Deleted orphaned blob {}", hash);
                        }
                    }

                    // Then drain any remaining items in the DB queue (safety net)
                    drain_cleanup_queue(&pool, &storage, batch_size).await;
                }
            }
            _ = cleanup_interval.tick() => {
                drain_cleanup_queue(&pool, &storage, batch_size).await;
            }
        }
    }

    info!("[StorageWorker] Stopped");
}

/// Drains the cleanup queue by processing batches until empty
async fn drain_cleanup_queue(pool: &sqlx::PgPool, storage: &FileStorageService, batch_size: i64) {
    loop {
        let mut conn = match pool.acquire().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("[StorageWorker] Failed to acquire connection: {}", e);
                break;
            }
        };

        match process_cleanup_batch(&mut conn, storage, None, batch_size).await {
            Ok(0) => break, // Queue is empty
            Ok(count) => {
                info!("[StorageWorker] Processed batch of {} hashes", count);
            }
            Err(e) => {
                warn!("[StorageWorker] Error processing cleanup batch: {}", e);
                break;
            }
        }
    }
}

/// Processes a batch of hashes from the cleanup queue
pub async fn process_cleanup_batch(
    conn: &mut sqlx::PgConnection,
    storage: &FileStorageService,
    workspace_id: Option<Uuid>,
    batch_size: i64,
) -> crate::error::Result<usize> {
    // Start a transaction to atomize queue claiming
    let mut tx = conn.begin().await.map_err(crate::error::Error::Sqlx)?;

    // Claim items to check.
    let items = files::claim_cleanup_batch(&mut *tx, workspace_id, batch_size).await?;
    let count = items.len();

    if count == 0 {
        tx.commit().await.map_err(crate::error::Error::Sqlx)?;
        return Ok(0);
    }

    for item in items {
        // Since hashes are salted with version_id, they are globally unique.
        // We can safely delete the physical blob immediately as no other version
        // could possibly reference this specific hash.
        if let Err(e) = storage.delete_archive_blob(item.workspace_id, &item.hash).await {
            warn!("[StorageWorker] Failed to delete blob {}: {}", item.hash, e);
        } else {
            info!("[StorageWorker] Deleted orphaned blob {}", item.hash);
        }
    }

    tx.commit().await.map_err(crate::error::Error::Sqlx)?;
    Ok(count)
}
