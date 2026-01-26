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

pub use context::{
    AttachmentManager, AttachmentKey, AttachmentValue, ESTIMATED_CHARS_PER_TOKEN,
    HistoryManager, PRIORITY_ESSENTIAL, PRIORITY_HIGH, PRIORITY_LOW, PRIORITY_MEDIUM,
};

#[cfg(test)]
mod tests;

use crate::{
    error::Result,
    models::chat::{ChatAttachment, ChatMessage, NewChatMessage},
    queries, DbConn,
};
use uuid::Uuid;
use sha2::{Sha256, Digest};

/// Default token limit for the context window in the MVP.
pub const DEFAULT_CONTEXT_TOKEN_LIMIT: usize = 4000;

/// Structured context for AI chat sessions.
///
/// Contains persona, conversation history, and file attachments separately,
/// allowing proper integration with AI frameworks like Rig.
///
/// Uses AttachmentManager and HistoryManager for sophisticated management:
/// - AttachmentManager: Priority-based pruning for file attachments
/// - HistoryManager: Token estimation and future pruning for conversations
#[derive(Debug, Clone)]
pub struct BuiltContext {
    /// System persona/instructions for the AI
    pub persona: String,
    /// History manager for conversation messages
    pub history: HistoryManager,
    /// Attachment manager for file attachments with priority-based pruning
    pub attachment_manager: AttachmentManager,
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
    /// Saves a message and updates the parent file's snapshot in a single transaction.
    /// Implements Write-Through Caching to ensure `read` and `grep` work on chat files.
    pub async fn save_message(
        conn: &mut DbConn,
        workspace_id: Uuid,
        new_msg: NewChatMessage,
    ) -> Result<ChatMessage> {
        let file_id = new_msg.file_id;
        
        // 1. Insert message into Source of Truth (chat_messages)
        let msg = queries::chat::insert_chat_message(conn, new_msg).await?;
        
        // 2. Snapshot the full state to file_versions (File View)
        Self::snapshot_chat_state(conn, workspace_id, file_id).await?;

        // 3. Touch file to update timestamp
        queries::files::touch_file(conn, file_id).await?;
        
        Ok(msg)
    }

    /// Snapshots the current chat state (messages + config) into the file_versions table.
    /// This enables 'read' and 'grep' tools to see the chat content.
    /// Uses In-Place Update optimization for the latest version to prevent history bloat.
    async fn snapshot_chat_state(conn: &mut DbConn, workspace_id: Uuid, chat_file_id: Uuid) -> Result<()> {
        // 1. Fetch all messages (Source of Truth)
        let messages = queries::chat::get_messages_by_file_id(conn, workspace_id, chat_file_id).await?;

        // 2. Get file to check latest version
        let file = queries::files::get_file_by_id(conn, chat_file_id).await?;
        
        // 3. Get existing config from latest version (or default)
        let agent_config = if let Some(_version_id) = file.latest_version_id {
            // We can reuse the query here, but we need to handle the case where it might fail if ID is bad (unlikely)
            if let Ok(version) = queries::files::get_latest_version(conn, chat_file_id).await {
                serde_json::from_value(version.app_data).unwrap_or_else(|_| crate::models::chat::AgentConfig {
                    agent_id: None,
                    model: "gpt-4o-mini".to_string(),
                    temperature: 0.7,
                    persona_override: None,
                })
            } else {
                 crate::models::chat::AgentConfig {
                    agent_id: None,
                    model: "gpt-4o-mini".to_string(),
                    temperature: 0.7,
                    persona_override: None,
                }
            }
        } else {
             crate::models::chat::AgentConfig {
                agent_id: None,
                model: "gpt-4o-mini".to_string(),
                temperature: 0.7,
                persona_override: None,
            }
        };

        // 4. Build session object
        let session = crate::models::chat::ChatSession {
            file_id: chat_file_id,
            agent_config: agent_config.clone(),
            messages,
        };

        let content_raw = serde_json::to_value(&session).map_err(crate::error::Error::Json)?;
        
        // 5. Calculate hash
        // Use a consistent serialization for hashing
        let content_str = serde_json::to_string(&content_raw).map_err(crate::error::Error::Json)?;
        let mut hasher = Sha256::new();
        hasher.update(content_str);
        let hash = hex::encode(hasher.finalize());
        
        // 6. Update or Create Version
        if let Some(version_id) = file.latest_version_id {
             // In-Place Update Optimization
             queries::files::update_version_content(conn, version_id, content_raw, hash).await?;
        } else {
             // Create new version (fallback)
             let version = queries::files::create_version(conn, crate::models::files::NewFileVersion {
                 file_id: chat_file_id,
                 workspace_id,
                 branch: "main".to_string(),
                 content_raw,
                 app_data: serde_json::to_value(agent_config).unwrap_or(serde_json::Value::Null),
                 hash,
                 author_id: file.author_id,
             }).await?;
             queries::files::update_latest_version_id(conn, chat_file_id, version.id).await?;
        }
        
        Ok(())
    }

    /// Builds the structured context for a chat session.
    ///
    /// Returns persona, history, and file attachments separately for proper
    /// integration with AI frameworks like Rig.
    ///
    /// Uses AttachmentManager internally for sophisticated attachment management:
    /// - Priority-based pruning for token limit optimization
    /// - Keyed addressability for efficient updates
    /// - Automatic token estimation
    pub async fn build_context(
        conn: &mut DbConn,
        workspace_id: Uuid,
        chat_file_id: Uuid,
        default_persona: &str,
        default_context_token_limit: usize,
    ) -> Result<BuiltContext> {
        // 1. Load Session Identity & History
        let messages = queries::chat::get_messages_by_file_id(conn, workspace_id, chat_file_id).await?;

        // 2. Hydrate Persona
        let persona = default_persona.to_string();

        // 3. Extract history (exclude current/last message which is the prompt)
        let history_messages = if messages.len() > 1 {
            messages[..messages.len() - 1].to_vec()
        } else {
            Vec::new()
        };
        let history = HistoryManager::new(history_messages);

        // 4. Hydrate Attachments using AttachmentManager
        let mut attachment_manager = AttachmentManager::new();

        if let Some(last_msg) = messages.last() {
            let metadata = &last_msg.metadata.0;
            for attachment in &metadata.attachments {
                if let ChatAttachment::File { file_id, .. } = attachment
                    && let Ok(file_with_content) =
                        crate::services::files::get_file_with_content(conn, *file_id).await
                {
                    // Security check: Ensure file belongs to the same workspace
                    if file_with_content.file.workspace_id == workspace_id {
                        let content = file_with_content.latest_version.content_raw.to_string();

                        // Estimate tokens (rough approximation: 4 chars per token)
                        let estimated_tokens = content.len() / ESTIMATED_CHARS_PER_TOKEN;

                        // Add to attachment manager with workspace file key
                        // Use MEDIUM priority for user-attached files
                        attachment_manager.add_fragment(
                            AttachmentKey::WorkspaceFile(*file_id),
                            AttachmentValue {
                                content,
                                priority: PRIORITY_MEDIUM,
                                tokens: estimated_tokens,
                                is_essential: false,
                            },
                        );
                    }
                }
            }
        }

        // 5. Optimize attachments for token limit
        // Keep only essential and high-priority files if we're over the limit
        attachment_manager.optimize_for_limit(default_context_token_limit);

        // 6. Sort by position for consistent rendering
        attachment_manager.sort_by_position();

        Ok(BuiltContext {
            persona,
            history,
            attachment_manager,
        })
    }
}
