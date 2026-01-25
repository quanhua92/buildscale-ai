//! Chat service with AI agent integration
//!
//! This module provides the Agentic Engine for AI-powered chat interactions with workspace files.
//! It integrates Rig.rs AI runtime with native workspace tools for autonomous file operations.
//!
//! # Architecture
//!
//! - **Rig Engine**: AI runtime (OpenAI GPT-4) with tool-calling capabilities
//! - **Rig Tools**: Thin adapters that expose workspace tools to the AI
//! - **Context Manager**: Manages conversation history and file attachments
//! - **Actor System**: Manages concurrent chat sessions with SSE streaming
//!
//! # Tool Behavior for AI
//!
//! The AI tools use smart content handling to optimize interactions:
//!
//! ## Document Files (Auto-Wrap/Unwrap)
//!
//! - **Write**: `"hello"` → stored as `{text: "hello"}` (auto-wrapped)
//! - **Read**: `{text: "hello"}` → returns `"hello"` (auto-unwrapped)
//! - **Why**: Simplifies AI input/output - no need for manual JSON wrapping
//!
//! ```json
//! // AI can write:
//! {"path": "/notes.md", "content": "Hello World"}
//!
//! // AI reads back:
//! {"content": "Hello World"}  // Not {"text": "Hello World"}
//! ```
//!
//! ## Other File Types (Raw JSONB)
//!
//! - **Canvas/Whiteboard/Chat**: No transformation
//! - **Write**: Must provide correct JSON structure
//! - **Read**: Returns exact stored JSON
//!
//! ```json
//! // AI must write canvas with full structure:
//! {
//!   "path": "/design.canvas",
//!   "content": {
//!     "elements": [{"type": "rect"}],
//!     "version": 1
//!   }
//! }
//!
//! // AI reads back same structure:
//! {"content": {"elements": [...], "version": 1}}
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use buildscale::services::chat::{rig_engine::RigService, ChatService};
//! use std::sync::Arc;
//! use uuid::Uuid;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Create Rig service
//! let rig_service = Arc::new(RigService::from_env());
//!
//! // 2. Build agent with tools (requires pool, workspace_id, user_id)
//! // let agent = rig_service.create_agent(&pool, workspace_id, user_id).await?;
//!
//! // 3. Execute tool calls
//! // let response = agent.chat("Create a file called hello.txt").await?;
//! # Ok(())
//! # }
//! ```

pub mod actor;
pub mod context;
pub mod registry;
pub mod rig_engine;
pub mod rig_tools;

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
            // Access metadata directly as ChatMessageMetadata
            let metadata = &last_msg.metadata.0;
            for attachment in &metadata.attachments {
                match attachment {
                    ChatAttachment::File { file_id, .. } => {
                        if let Ok(file_with_content) = crate::services::files::get_file_with_content(conn, *file_id).await {
                            // Security check: Ensure file belongs to the same workspace
                            if file_with_content.file.workspace_id == workspace_id {
                                let content = format_file_fragment(
                                    &file_with_content.file.path,
                                    &file_with_content.latest_version.content_raw.to_string()
                                );
                                
                                manager.add_fragment(
                                    ContextKey::WorkspaceFile(*file_id),
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

        // 5. Engineering: Sort and Render
        manager.sort_by_position();
        manager.optimize_for_limit(DEFAULT_CONTEXT_TOKEN_LIMIT);

        Ok(manager.render())
    }
}
