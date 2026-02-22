//! Stream processing utility functions for ChatActor
//!
//! This module contains standalone helper functions for stream processing operations.

use crate::models::chat::{ChatMessageMetadata, ChatMessageRole};
use crate::services::chat::ChatService;
use crate::services::storage::FileStorageService;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Flush the reasoning buffer to the database.
///
/// This function extracts the reasoning buffer from the actor state,
/// aggregates it, and saves it as a chat message with reasoning metadata.
pub async fn flush_reasoning_buffer(
    state: &Arc<Mutex<crate::services::chat::states::SharedActorState>>,
    chat_id: Uuid,
    workspace_id: Uuid,
    storage: &Arc<FileStorageService>,
    conn: &mut sqlx::PgConnection,
) -> crate::error::Result<()> {
    let (buffer, reasoning_id) = {
        let mut state_guard = state.lock().await;
        if state_guard.reasoning_buffer.is_empty() {
            return Ok(());
        }
        // Atomically take the buffer, leaving an empty one in its place to prevent a race condition.
        let buffer = std::mem::take(&mut state_guard.reasoning_buffer);
        let reasoning_id = state_guard.ensure_reasoning_id();
        (buffer, reasoning_id)
    };

    let aggregated_reasoning = buffer.join("");
    if aggregated_reasoning.is_empty() {
        return Ok(());
    }

    tracing::debug!(
        chat_id = %chat_id,
        reasoning_len = aggregated_reasoning.len(),
        reasoning_id = %reasoning_id,
        "[ChatActor] Flushing aggregated reasoning buffer to DB"
    );

    let metadata = ChatMessageMetadata {
        message_type: Some("reasoning_complete".to_string()),
        reasoning_id: Some(reasoning_id.clone()),
        ..Default::default()
    };

    ChatService::save_stream_event(
        conn,
        storage,
        workspace_id,
        chat_id,
        ChatMessageRole::Assistant,
        aggregated_reasoning,
        metadata,
    )
    .await?;

    tracing::debug!(
        chat_id = %chat_id,
        reasoning_id = %reasoning_id,
        "[ChatActor] Reasoning buffer flushed successfully"
    );

    Ok(())
}
