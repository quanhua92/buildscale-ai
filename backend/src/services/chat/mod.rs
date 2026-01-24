pub mod context;
#[cfg(test)]
mod tests;

use crate::{
    error::Result,
    models::chat::{ChatAttachment, ChatMessage, NewChatMessage},
    queries, DbConn,
};
use self::context::{
    format_file_fragment, format_history_fragment, ContextKey, ContextManager, ContextValue,
    ESTIMATED_CHARS_PER_TOKEN, PRIORITY_ESSENTIAL, PRIORITY_HIGH, PRIORITY_MEDIUM,
};
use uuid::Uuid;

/// Default token limit for the context window in the MVP.
pub const DEFAULT_CONTEXT_TOKEN_LIMIT: usize = 4000;

pub struct ChatService;

impl ChatService {
    /// Saves a message and updates the parent file's timestamp in a single transaction.
    pub async fn save_message(
        conn: &mut DbConn,
        _workspace_id: Uuid,
        new_msg: NewChatMessage,
    ) -> Result<ChatMessage> {
        let file_id = new_msg.file_id;
        
        // Orchestrate persistence across tables
        let msg = queries::chat::insert_chat_message(conn, new_msg).await?;
        queries::files::touch_file(conn, file_id).await?;
        
        Ok(msg)
    }

    /// Builds the full context for a chat session by hydrating all fragments.
    pub async fn build_context(
        conn: &mut DbConn,
        workspace_id: Uuid,
        chat_file_id: Uuid,
    ) -> Result<String> {
        let mut manager = ContextManager::new();

        // 1. Load Session Identity & History
        let messages = queries::chat::get_messages_by_file_id(conn, workspace_id, chat_file_id).await?;
        
        // 2. Hydrate Persona (from AgentConfig in file app_data - logic to be refined in Phase 1.4)
        // For now, we use a placeholder or system default
        manager.add_fragment(
            ContextKey::SystemPersona,
            ContextValue {
                content: "You are BuildScale AI, a professional software engineering assistant.".to_string(),
                priority: PRIORITY_ESSENTIAL,
                tokens: 15, // Fixed system prompt estimation
                is_essential: true,
            },
        );

        // 3. Hydrate History
        if !messages.is_empty() {
            let history_content = format_history_fragment(&messages);
            manager.add_fragment(
                ContextKey::ChatHistory,
                ContextValue {
                    tokens: history_content.len() / ESTIMATED_CHARS_PER_TOKEN,
                    content: history_content,
                    priority: PRIORITY_MEDIUM,
                    is_essential: false,
                },
            );
        }

        // 4. Hydrate Attachments from the LATEST message
        if let Some(last_msg) = messages.last() {
            // Attempt to parse metadata for attachments
            // Metadata is serde_json::Value in the model
            if let Ok(metadata) = serde_json::from_value::<crate::models::chat::ChatMessageMetadata>(last_msg.metadata.clone()) {
                for attachment in metadata.attachments {
                    match attachment {
                        ChatAttachment::File { file_id, .. } => {
                            if let Ok(file_with_content) = crate::services::files::get_file_with_content(conn, file_id).await {
                                // Security check: Ensure file belongs to the same workspace
                                if file_with_content.file.workspace_id == workspace_id {
                                    let content = format_file_fragment(
                                        &file_with_content.file.path,
                                        &file_with_content.latest_version.content_raw.to_string()
                                    );
                                    
                                    manager.add_fragment(
                                        ContextKey::WorkspaceFile(file_id),
                                        ContextValue {
                                            tokens: content.len() / ESTIMATED_CHARS_PER_TOKEN,
                                            content,
                                            priority: PRIORITY_HIGH,
                                            is_essential: false,
                                        },
                                    );
                                }
                            }
                        },
                        _ => {} // Other attachments handled in Phase 2
                    }
                }
            }
        }

        // 5. Engineering: Sort and Render
        manager.sort_by_position();
        manager.optimize_for_limit(DEFAULT_CONTEXT_TOKEN_LIMIT);

        Ok(manager.render())
    }
}
