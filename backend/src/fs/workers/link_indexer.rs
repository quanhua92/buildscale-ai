//! Background worker for indexing wikilinks from file content
//!
//! This worker listens for file change messages and updates the links table.
//! Uses batch processing: accumulates messages and processes in batches for efficiency.

use crate::state::LinkIndexMessage;
use crate::config::StorageConfig;
use super::super::storage::FileStorageService;
use super::super::parsers::extract_links;
use sqlx::Acquire;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::interval;
use tokio::sync::mpsc;
use tracing::{info, warn, debug};
use uuid::Uuid;

/// Background worker that indexes wikilinks from file content
///
/// Listens for file change messages and updates the links table.
/// Uses batch processing: accumulates messages and processes in batches.
pub async fn link_indexer_worker(
    pool: sqlx::PgPool,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    mut link_index_rx: mpsc::UnboundedReceiver<LinkIndexMessage>,
    storage_config: StorageConfig,
) {
    let storage = FileStorageService::new(&storage_config.base_path);
    let mut batch_interval = interval(Duration::from_millis(100)); // Process batch every 100ms

    info!("[LinkIndexer] Started (listening for file changes)");

    // Accumulate messages for batch processing
    // workspace_id -> file_ids
    let mut pending: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("[LinkIndexer] Shutting down");
                // Process remaining pending items before shutdown
                if !pending.is_empty() {
                    process_batch(&pool, &storage, &pending).await;
                }
                break;
            }
            message = link_index_rx.recv() => {
                if let Some(msg) = message {
                    pending
                        .entry(msg.workspace_id)
                        .or_default()
                        .push(msg.file_id);

                    // Limit batch size to prevent memory issues
                    let total: usize = pending.values().map(|v| v.len()).sum();
                    if total >= 100 {
                        process_batch(&pool, &storage, &pending).await;
                        pending.clear();
                    }
                }
            }
            _ = batch_interval.tick() => {
                if !pending.is_empty() {
                    process_batch(&pool, &storage, &pending).await;
                    pending.clear();
                }
            }
        }
    }

    info!("[LinkIndexer] Stopped");
}

/// Process a batch of file link updates
async fn process_batch(
    pool: &sqlx::PgPool,
    storage: &FileStorageService,
    pending: &HashMap<Uuid, Vec<Uuid>>,
) {
    for (workspace_id, file_ids) in pending {
        for file_id in file_ids {
            if let Err(e) = index_file_links(pool, storage, *workspace_id, *file_id).await {
                warn!("[LinkIndexer] Failed to index file {}: {}", file_id, e);
            }
        }
    }
}

/// Index links for a single file
async fn index_file_links(
    pool: &sqlx::PgPool,
    storage: &FileStorageService,
    workspace_id: Uuid,
    file_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Get file info
    let mut conn = pool.acquire().await
        .map_err(|e| format!("Failed to acquire connection: {}", e))?;

    let file = sqlx::query!(
        "SELECT path, hash FROM files WHERE id = $1",
        file_id
    )
    .fetch_optional(&mut *conn)
    .await?;

    let file = match file {
        Some(f) => f,
        None => {
            debug!("[LinkIndexer] File {} not found, skipping", file_id);
            return Ok(());
        }
    };

    // 2. Skip folders (no content to parse)
    if file.hash.is_none() {
        debug!("[LinkIndexer] File {} is a folder or has no content, skipping", file_id);
        return Ok(());
    }

    // 3. Read file content
    let content_bytes = storage.read_file(workspace_id, &file.path).await?;
    let content = String::from_utf8(content_bytes).unwrap_or_default();

    // 4. Extract links from content
    let links = extract_links(&content);
    let link_count = links.len();

    // 5. Update links table (transaction)
    let mut tx = conn.begin().await?;

    // Delete existing links for this file
    sqlx::query!(
        "DELETE FROM links WHERE source_file_id = $1",
        file_id
    )
    .execute(&mut *tx)
    .await?;

    // Insert new links (store target names in lowercase for case-insensitive matching)
    for link in links {
        let target_name = link.to_lowercase();
        sqlx::query!(
            r#"
            INSERT INTO links (workspace_id, source_file_id, target_name)
            VALUES ($1, $2, $3)
            ON CONFLICT (workspace_id, source_file_id, target_name) DO NOTHING
            "#,
            workspace_id,
            file_id,
            target_name
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    debug!("[LinkIndexer] Indexed {} links for file {}", link_count, file_id);

    Ok(())
}
