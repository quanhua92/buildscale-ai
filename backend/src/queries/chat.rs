use crate::{
    error::{Error, Result},
    models::chat::{ChatMessage, ChatMessageRole, NewChatMessage},
    DbConn,
};
use uuid::Uuid;

/// Inserts a new chat message into the database.
pub async fn insert_chat_message(conn: &mut DbConn, new_msg: NewChatMessage) -> Result<ChatMessage> {
    let msg = sqlx::query_as!(
        ChatMessage,
        r#"
        INSERT INTO chat_messages (file_id, workspace_id, role, content, metadata)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING 
            id, 
            file_id, 
            workspace_id, 
            role as "role: ChatMessageRole", 
            content, 
            metadata, 
            created_at, 
            updated_at, 
            deleted_at
        "#,
        new_msg.file_id,
        new_msg.workspace_id,
        new_msg.role as ChatMessageRole,
        new_msg.content,
        new_msg.metadata
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(msg)
}

/// Retrieves all non-deleted messages for a specific chat file.
pub async fn get_messages_by_file_id(
    conn: &mut DbConn,
    workspace_id: Uuid,
    file_id: Uuid,
) -> Result<Vec<ChatMessage>> {
    let messages = sqlx::query_as!(
        ChatMessage,
        r#"
        SELECT 
            id, 
            file_id, 
            workspace_id, 
            role as "role: ChatMessageRole", 
            content, 
            metadata, 
            created_at, 
            updated_at, 
            deleted_at
        FROM chat_messages
        WHERE file_id = $1 AND workspace_id = $2 AND deleted_at IS NULL
        ORDER BY created_at ASC
        "#,
        file_id,
        workspace_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(messages)
}

/// Updates the content and metadata of an existing chat message.
pub async fn update_chat_message(
    conn: &mut DbConn,
    workspace_id: Uuid,
    message_id: Uuid,
    content: String,
    metadata: serde_json::Value,
) -> Result<ChatMessage> {
    let msg = sqlx::query_as!(
        ChatMessage,
        r#"
        UPDATE chat_messages
        SET content = $3, metadata = $4, updated_at = NOW()
        WHERE id = $1 AND workspace_id = $2
        RETURNING 
            id, 
            file_id, 
            workspace_id, 
            role as "role: ChatMessageRole", 
            content, 
            metadata, 
            created_at, 
            updated_at, 
            deleted_at
        "#,
        message_id,
        workspace_id,
        content,
        metadata
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(msg)
}

/// Soft-deletes a specific chat message.
pub async fn soft_delete_chat_message(
    conn: &mut DbConn,
    workspace_id: Uuid,
    message_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE chat_messages
        SET deleted_at = NOW(), updated_at = NOW()
        WHERE id = $1 AND workspace_id = $2
        "#,
        message_id,
        workspace_id
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}

/// Soft-deletes all messages associated with a chat file.
pub async fn soft_delete_all_messages_by_file_id(
    conn: &mut DbConn,
    workspace_id: Uuid,
    file_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE chat_messages
        SET deleted_at = NOW(), updated_at = NOW()
        WHERE file_id = $1 AND workspace_id = $2
        "#,
        file_id,
        workspace_id
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}
