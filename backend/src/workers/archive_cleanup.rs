use crate::queries::files;
use crate::services::storage::FileStorageService;
use crate::Config;
use crate::state::ArchiveCleanupMessage;
use std::time::Duration;
use tokio::time::interval;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use sqlx::Connection;

/// Background worker that periodically cleans up orphaned archive blobs
///
/// Runs every hour to check the cleanup queue for stale hashes
pub async fn archive_cleanup_worker(
    pool: sqlx::PgPool,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    mut archive_cleanup_rx: mpsc::UnboundedReceiver<ArchiveCleanupMessage>,
) {
    let mut cleanup_interval = interval(Duration::from_secs(3600)); // Every hour
    info!("[StorageWorker] Started (runs every hour or on-demand)");

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("[StorageWorker] Shutting down");
                break;
            }
            message = archive_cleanup_rx.recv() => {
                if let Some(_msg) = message {
                    let mut conn = match pool.acquire().await {
                        Ok(conn) => conn,
                        Err(e) => {
                            error!("[StorageWorker] Failed to acquire connection for immediate cleanup: {}", e);
                            continue;
                        }
                    };

                    let config = match Config::load() {
                        Ok(cfg) => cfg,
                        Err(e) => {
                            error!("[StorageWorker] Failed to load config: {}", e);
                            continue;
                        }
                    };

                    let storage = FileStorageService::new(&config.storage.base_path);
                    
                    // Work until queue is empty
                    loop {
                        match process_cleanup_batch(&mut conn, &storage).await {
                            Ok(0) => break, // Empty
                            Ok(count) => {
                                info!("[StorageWorker] Processed batch of {} hashes from cleanup queue", count);
                            }
                            Err(e) => {
                                warn!("[StorageWorker] Failed to process cleanup batch: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            _ = cleanup_interval.tick() => {
                let mut conn = match pool.acquire().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("[StorageWorker] Failed to acquire database connection for cleanup: {}", e);
                        continue;
                    }
                };

                let config = match Config::load() {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        error!("[StorageWorker] Failed to load config for cleanup: {}", e);
                        continue;
                    }
                };

                let storage = FileStorageService::new(&config.storage.base_path);

                // Work until queue is empty
                loop {
                    match process_cleanup_batch(&mut conn, &storage).await {
                        Ok(0) => break,
                        Ok(count) => {
                            info!("[StorageWorker] Processed batch of {} hashes from cleanup queue", count);
                        }
                        Err(e) => {
                            warn!("[StorageWorker] Failed to process cleanup batch: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }

    info!("[StorageWorker] Stopped");
}

/// Processes a batch of hashes from the cleanup queue
pub async fn process_cleanup_batch(
    conn: &mut sqlx::PgConnection,
    storage: &FileStorageService,
) -> crate::error::Result<usize> {
    // Start a transaction to atomize queue claiming and reference checking
    let mut tx = conn.begin().await.map_err(crate::error::Error::Sqlx)?;

    // Claim 10 items to keep transaction time short
    let items = files::claim_cleanup_batch(&mut *tx, 10).await?;
    let count = items.len();

    if count == 0 {
        tx.commit().await.map_err(crate::error::Error::Sqlx)?;
        return Ok(0);
    }

    for item in items {
        // Check if hash is still referenced (deduplication)
        // Since we are in a transaction, we should use FOR SHARE on any referenced rows 
        // if we wanted to be 100% race-proof, but standard existence check is usually enough
        // given the append-only nature of versions.
        match files::is_hash_referenced(&mut *tx, &item.hash).await {
            Ok(referenced) => {
                if !referenced {
                    // Delete from disk
                    if let Err(e) = storage.delete_archive_blob(item.workspace_id, &item.hash).await {
                        warn!("[StorageWorker] Failed to delete blob {}: {}", item.hash, e);
                    } else {
                        info!("[StorageWorker] Deleted orphaned blob {}", item.hash);
                    }
                }
            }
            Err(e) => {
                warn!("[StorageWorker] Failed to check reference for {}: {}", item.hash, e);
                // We've already claimed/deleted from queue in this tx, 
                // so if we fail here, we might want to rollback.
                return Err(e);
            }
        }
    }

    tx.commit().await.map_err(crate::error::Error::Sqlx)?;
    Ok(count)
}
