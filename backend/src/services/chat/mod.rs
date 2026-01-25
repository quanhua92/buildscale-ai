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
use uuid::Uuid;

/// Default token limit for the context window in the MVP.
pub const DEFAULT_CONTEXT_TOKEN_LIMIT: usize = 4000;

/// Structured context for AI chat sessions.
///
/// Contains persona, conversation history, and file attachments separately,
/// allowing proper integration with AI frameworks like Rig.
#[derive(Debug, Clone)]
pub struct BuiltContext {
    /// System persona/instructions for the AI
    pub persona: String,
    /// Conversation history (excluding current message)
    pub history: Vec<ChatMessage>,
    /// File attachments with their content
    pub attachments: Vec<FileAttachment>,
}

/// A file attachment with its content for context.
#[derive(Debug, Clone)]
pub struct FileAttachment {
    pub file_id: Uuid,
    pub path: String,
    pub content: String,
}

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

    /// Builds the structured context for a chat session.
    ///
    /// Returns persona, history, and file attachments separately for proper
    /// integration with AI frameworks like Rig.
    pub async fn build_context(
        conn: &mut DbConn,
        workspace_id: Uuid,
        chat_file_id: Uuid,
    ) -> Result<BuiltContext> {
        // 1. Load Session Identity & History
        let messages = queries::chat::get_messages_by_file_id(conn, workspace_id, chat_file_id).await?;

        // 2. Hydrate Persona
        let persona = "You are BuildScale AI, a professional software engineering assistant.".to_string();

        // 3. Extract history (exclude current/last message which is the prompt)
        let history = if messages.len() > 1 {
            messages[..messages.len() - 1].to_vec()
        } else {
            Vec::new()
        };

        // 4. Hydrate Attachments from the LATEST message
        let mut attachments = Vec::new();

        if let Some(last_msg) = messages.last() {
            let metadata = &last_msg.metadata.0;
            for attachment in &metadata.attachments {
                match attachment {
                    ChatAttachment::File { file_id, .. } => {
                        if let Ok(file_with_content) =
                            crate::services::files::get_file_with_content(conn, *file_id).await
                        {
                            // Security check: Ensure file belongs to the same workspace
                            if file_with_content.file.workspace_id == workspace_id {
                                attachments.push(FileAttachment {
                                    file_id: *file_id,
                                    path: file_with_content.file.path.clone(),
                                    content: file_with_content
                                        .latest_version
                                        .content_raw
                                        .to_string(),
                                });
                            }
                        }
                    }
                    _ => {} // Other attachments handled in Phase 2
                }
            }
        }

        // 5. Apply token limit optimization (if context is too large)
        // For now, we keep all history but could optimize in the future
        // TODO: Implement smart history truncation based on token limits

        Ok(BuiltContext {
            persona,
            history,
            attachments,
        })
    }
}
