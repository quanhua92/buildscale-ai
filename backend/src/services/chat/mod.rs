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
//! # Model Management
//!
//! The chat system supports dynamic model switching:
//!
//! - **Available Models**: `gpt-5`, `gpt-5-mini`, `gpt-5-nano`, `gpt-5.1`, `gpt-4o`, `gpt-4o-mini`
//! - **Default Model**: `gpt-5-mini` (used if not specified)
//! - **Model Persistence**: Model is stored per chat in file version's `app_data`
//! - **Model Switching**: Use `ChatService::update_chat_model()` to change models
//!
//! ## Example: Switch Models Mid-Chat
//!
//! ```text
//! // Chat started with gpt-5-mini (default)
//! // ... exchange messages ...
//!
//! // Switch to gpt-5 for more complex reasoning
//! ChatService::update_chat_model(
//!     &mut conn,
//!     workspace_id,
//!     chat_file_id,
//!     "gpt-5".to_string()
//! ).await?;
//! ```
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
    error::{Error, Result},
    models::chat::{AgentConfig, ChatAttachment, ChatMessage, NewChatMessage, DEFAULT_CHAT_MODEL},
    queries, DbConn,
};
use uuid::Uuid;

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
    /// Saves a message and appends it to the disk file (Hybrid Persistence).
    /// - DB: Structured storage for O(1) query and context construction.
    /// - Disk: Markdown log for human readability and file system tools.
    pub async fn save_message(
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        new_msg: NewChatMessage,
    ) -> Result<ChatMessage> {
        let file_id = new_msg.file_id;
        
        // 1. Insert message into Source of Truth (chat_messages)
        let msg = queries::chat::insert_chat_message(conn, new_msg).await?;
        
        // 2. Append to Disk (File View)
        // Retrieve file path first
        let file = queries::files::get_file_by_id(conn, file_id).await?;

        let markdown_entry = format_message_as_markdown(&msg);
        // Use flat storage path for consistency with file storage changes
        // Files are stored at /{slug} instead of their full logical path
        let storage_path = format!("/{}", file.slug);
        storage.append_to_file(workspace_id, &storage_path, &markdown_entry).await?;

        // 3. Touch file to update timestamp
        queries::files::touch_file(conn, file_id).await?;
        
        Ok(msg)
    }

    /// Updates the model for a chat session in app_data.
    pub async fn update_chat_model(
        conn: &mut DbConn,
        workspace_id: Uuid,
        chat_file_id: Uuid,
        new_model: String,
    ) -> Result<()> {
        // 1. Get current version to extract existing agent_config
        let version = queries::files::get_latest_version(conn, chat_file_id).await?;
        let mut agent_config: AgentConfig = serde_json::from_value(version.app_data)
            .unwrap_or_else(|_| AgentConfig {
                agent_id: None,
                model: DEFAULT_CHAT_MODEL.to_string(),
                temperature: 0.7,
                persona_override: None,
                previous_response_id: None,
            });

        // 2. Update the model field
        agent_config.model = new_model.clone();

        // 3. Create new version with updated agent_config
        let new_app_data = serde_json::to_value(agent_config).map_err(Error::Json)?;

        let new_version = queries::files::create_version(conn, crate::models::files::NewFileVersion {
            file_id: chat_file_id,
            workspace_id,
            branch: "main".to_string(),
            app_data: new_app_data,
            hash: "model-update".to_string(),
            author_id: None,
        }).await?;

        queries::files::update_latest_version_id(conn, chat_file_id, new_version.id).await?;

        tracing::info!("[ChatService] Updated model for chat {} to {}", chat_file_id, new_model);

        Ok(())
    }

    /// Retrieves the full chat session including configuration and message history.
    pub async fn get_chat_session(
        conn: &mut DbConn,
        workspace_id: Uuid,
        chat_file_id: Uuid,
    ) -> Result<crate::models::chat::ChatSession> {
        // 1. Verify file exists and is a chat
        let file = queries::files::get_file_by_id(conn, chat_file_id).await?;
        if file.workspace_id != workspace_id {
            return Err(crate::error::Error::NotFound(format!("Chat not found: {}", chat_file_id)));
        }
        if !matches!(file.file_type, crate::models::files::FileType::Chat) {
            return Err(crate::error::Error::Validation(crate::error::ValidationErrors::Single {
                field: "chat_id".to_string(),
                message: "File is not a chat".to_string(),
            }));
        }

        // 2. Fetch all messages
        let messages = queries::chat::get_messages_by_file_id(conn, workspace_id, chat_file_id).await?;

        // 3. Get existing config from latest version (or default)
        let agent_config = if let Some(_version_id) = file.latest_version_id {
            if let Ok(version) = queries::files::get_latest_version(conn, chat_file_id).await {
                serde_json::from_value(version.app_data).unwrap_or_else(|_| crate::models::chat::AgentConfig {
                    agent_id: None,
                    model: DEFAULT_CHAT_MODEL.to_string(),
                    temperature: 0.7,
                    persona_override: None,
                    previous_response_id: None,
                })
            } else {
                 crate::models::chat::AgentConfig {
                    agent_id: None,
                    model: DEFAULT_CHAT_MODEL.to_string(),
                    temperature: 0.7,
                    persona_override: None,
                    previous_response_id: None,
                }
            }
        } else {
             crate::models::chat::AgentConfig {
                agent_id: None,
                model: DEFAULT_CHAT_MODEL.to_string(),
                temperature: 0.7,
                persona_override: None,
                previous_response_id: None,
            }
        };

        Ok(crate::models::chat::ChatSession {
            file_id: chat_file_id,
            agent_config,
            messages,
        })
    }

    /// Builds the structured context for a chat session.
    pub async fn build_context(
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
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
                        crate::services::files::get_file_with_content(conn, storage, *file_id).await
                {
                    // Security check: Ensure file belongs to the same workspace
                    if file_with_content.file.workspace_id == workspace_id {
                        let content = file_with_content.content.to_string();

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

fn format_message_as_markdown(msg: &ChatMessage) -> String {
    let timestamp = msg.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let role = match msg.role {
        crate::models::chat::ChatMessageRole::User => "User",
        crate::models::chat::ChatMessageRole::Assistant => "Assistant",
        crate::models::chat::ChatMessageRole::System => "System",
        crate::models::chat::ChatMessageRole::Tool => "Tool",
    };
    
    format!("\n\n### {} ({})\n\n{}\n", role, timestamp, msg.content)
}
