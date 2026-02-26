//! Background worker for indexing tags from file content
//!
//! This worker listens for file change messages and updates the tags table.
//! Uses batch processing: accumulates messages and processes in batches for efficiency.

use crate::services::storage::FileStorageService;
use crate::state::TagIndexMessage;
use crate::parsers::extract_tags;
use sqlx::Acquire;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::interval;
use tokio::sync::mpsc;
use tracing::{info, warn, debug};
use uuid::Uuid;

/// Background worker that indexes tags from file content
///
/// Listens for file change messages and updates the tags table.
/// Uses batch processing: accumulates messages and processes in batches.
pub async fn tag_indexer_worker(
    pool: sqlx::PgPool,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    mut tag_index_rx: mpsc::UnboundedReceiver<TagIndexMessage>,
    storage_config: crate::config::StorageConfig,
) {
    let storage = FileStorageService::new(&storage_config.base_path);
    let mut batch_interval = interval(Duration::from_millis(100)); // Process batch every 100ms

    info!("[TagIndexer] Started (listening for file changes)");

    // Accumulate messages for batch processing
    // workspace_id -> file_ids
    let mut pending: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("[TagIndexer] Shutting down");
                // Process remaining pending items before shutdown
                if !pending.is_empty() {
                    process_batch(&pool, &storage, &pending).await;
                }
                break;
            }
            message = tag_index_rx.recv() => {
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

    info!("[TagIndexer] Stopped");
}

/// Process a batch of file tag updates
async fn process_batch(
    pool: &sqlx::PgPool,
    storage: &FileStorageService,
    pending: &HashMap<Uuid, Vec<Uuid>>,
) {
    for (workspace_id, file_ids) in pending {
        for file_id in file_ids {
            if let Err(e) = index_file_tags(pool, storage, *workspace_id, *file_id).await {
                warn!("[TagIndexer] Failed to index file {}: {}", file_id, e);
            }
        }
    }
}

/// Index tags for a single file
async fn index_file_tags(
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
            debug!("[TagIndexer] File {} not found, skipping", file_id);
            return Ok(());
        }
    };

    // 2. Skip folders (no content to parse)
    if file.hash.is_none() {
        debug!("[TagIndexer] File {} is a folder or has no content, skipping", file_id);
        return Ok(());
    }

    // 3. Read file content
    let content_bytes = storage.read_file(workspace_id, &file.path).await?;
    let content = String::from_utf8(content_bytes).unwrap_or_default();

    // 4. Extract tags from content
    let tags = extract_tags(&content);
    let tag_count = tags.len();

    // 5. Update tags table (transaction)
    let mut tx = conn.begin().await?;

    // Delete existing tags for this file
    sqlx::query!(
        "DELETE FROM tags WHERE file_id = $1",
        file_id
    )
    .execute(&mut *tx)
    .await?;

    // Insert new tags
    for tag in tags {
        sqlx::query!(
            r#"
            INSERT INTO tags (workspace_id, file_id, tag)
            VALUES ($1, $2, $3)
            ON CONFLICT (workspace_id, file_id, tag) DO NOTHING
            "#,
            workspace_id,
            file_id,
            tag
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    debug!("[TagIndexer] Indexed {} tags for file {}", tag_count, file_id);

    Ok(())
}
