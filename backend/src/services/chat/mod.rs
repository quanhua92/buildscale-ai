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
//! // 2. Build agent with tools (requires pool, workspace_id, chat_id, user_id)
//! // let agent = rig_service.create_agent(&pool, workspace_id, chat_id, user_id, session, &ai_config).await?;
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
pub mod sync;

pub use context::{
    AttachmentManager, AttachmentKey, AttachmentValue, ESTIMATED_CHARS_PER_TOKEN,
    HistoryManager, PRIORITY_ESSENTIAL, PRIORITY_HIGH, PRIORITY_LOW, PRIORITY_MEDIUM,
};

pub use sync::{ChatFrontmatter, YamlFrontmatter};

#[cfg(test)]
mod tests;

use crate::{
    error::{Error, Result},
    models::chat::{AgentConfig, ChatAttachment, ChatMessage, ChatMessageMetadata, ChatMessageRole, NewChatMessage, DEFAULT_CHAT_MODEL},
    queries, DbConn,
};
use uuid::Uuid;

/// Default token limit for the context window in the MVP.
pub const DEFAULT_CONTEXT_TOKEN_LIMIT: usize = 4000;

/// Max length for tool output before truncation (2KB)
const MAX_TOOL_OUTPUT_LENGTH: usize = 2048;
/// Max length for 'write' tool content arg (1KB)
const MAX_WRITE_CONTENT_LENGTH: usize = 1000;
/// Max length for 'edit' tool diff args (500 chars)
const MAX_EDIT_DIFF_LENGTH: usize = 500;
/// Number of items to preview for 'ls' tool
const LS_PREVIEW_ITEMS: usize = 50;
/// Number of matches to preview for 'grep' tool
const GREP_PREVIEW_MATCHES: usize = 20;
/// Number of lines to preview for 'read' tool
const READ_PREVIEW_LINES: usize = 5;

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
    ///
    /// Note: Reasoning messages (message_type="reasoning_complete") are saved to DB
    /// for audit purposes but NOT written to .chat files to avoid cluttering them.
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
        // Skip writing reasoning messages to .chat file - they're only for audit/debug in DB
        let is_reasoning = msg.metadata.message_type.as_ref()
            .map(|t| t == "reasoning_complete")
            .unwrap_or(false);

        if !is_reasoning {
            // Retrieve file path first
            let file = queries::files::get_file_by_id(conn, file_id).await?;

            let markdown_entry = format_message_as_markdown(&msg);
            // Use full hierarchical path for consistency with file storage
            storage.append_to_file(workspace_id, &file.path, &markdown_entry).await?;

            // 3. Touch file to update timestamp
            queries::files::touch_file(conn, file_id).await?;
        }

        Ok(msg)
    }

    /// Saves a streaming event as a ChatMessage (for audit trail persistence).
    ///
    /// This method is used to persist individual streaming events like reasoning chunks,
    /// tool calls, and tool results. It accepts a ChatMessageRole and metadata directly
    /// and delegates to `save_message` for hybrid persistence (DB + disk).
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `storage` - File storage service
    /// * `workspace_id` - Workspace owning the chat
    /// * `file_id` - Chat file ID
    /// * `role` - Message role (typically Tool or Assistant for streaming events)
    /// * `content` - Text content of the event
    /// * `metadata` - Structured metadata (message_type, reasoning_id, tool_* fields)
    pub async fn save_stream_event(
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        file_id: Uuid,
        role: ChatMessageRole,
        content: String,
        metadata: ChatMessageMetadata,
    ) -> Result<ChatMessage> {
        Self::save_message(
            conn,
            storage,
            workspace_id,
            NewChatMessage {
                file_id,
                workspace_id,
                role,
                content,
                metadata: sqlx::types::Json(metadata),
            },
        )
        .await
    }

    /// Summarizes tool inputs (arguments) to prevent database bloat.
    /// Truncates long string fields like 'content', 'old_string', 'new_string'.
    pub fn summarize_tool_inputs(
        tool_name: &str,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        // Clone args to modify
        let mut summary = args.clone();

        let Some(obj) = summary.as_object_mut() else {
            // If args is not an object, return as is (rare)
            return summary;
        };

        // Truncate specific fields based on tool
        match tool_name {
            "write" => {
                if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                    if content.len() > MAX_WRITE_CONTENT_LENGTH {
                        let preview = Self::extract_string_preview(content, 20);
                        obj.insert(
                            "content".to_string(),
                            serde_json::json!(format!("{}... [truncated, size={}]", preview, content.len())),
                        );
                    }
                }
            }
            "edit" => {
                for field in ["old_string", "new_string"] {
                    if let Some(val) = obj.get(field).and_then(|v| v.as_str()) {
                        if val.len() > MAX_EDIT_DIFF_LENGTH {
                            let preview = Self::extract_string_preview(val, 10);
                            obj.insert(
                                field.to_string(),
                                serde_json::json!(format!("{}... [truncated]", preview)),
                            );
                        }
                    }
                }
            }
            _ => {}
        }

        summary
    }

    /// Summarizes tool outputs (results) to prevent database bloat.
    /// Handles 'read', 'ls', 'grep' specifically, and generic truncation for others.
    pub fn summarize_tool_outputs(tool_name: &str, output: &str) -> String {
        // If output is small, keep it all
        if output.len() < MAX_TOOL_OUTPUT_LENGTH {
            return output.to_string();
        }

        match tool_name {
            "read" => {
                let line_count = output.lines().count();
                let byte_count = output.len();
                let preview = output.lines().take(READ_PREVIEW_LINES).collect::<Vec<_>>().join("\n");
                format!(
                    "[Read {} bytes, {} lines. Preview:]\n{}\n... [truncated]",
                    byte_count, line_count, preview
                )
            }
            "ls" => {
                let lines: Vec<&str> = output.lines().collect();
                if lines.len() > LS_PREVIEW_ITEMS {
                    let preview = lines.iter().take(LS_PREVIEW_ITEMS).cloned().collect::<Vec<_>>().join("\n");
                    format!("{}\n... ({} more items)", preview, lines.len() - LS_PREVIEW_ITEMS)
                } else {
                    output.to_string()
                }
            }
            "grep" => {
                let lines: Vec<&str> = output.lines().collect();
                if lines.len() > GREP_PREVIEW_MATCHES {
                    let preview = lines.iter().take(GREP_PREVIEW_MATCHES).cloned().collect::<Vec<_>>().join("\n");
                    format!("{}\n... ({} more matches)", preview, lines.len() - GREP_PREVIEW_MATCHES)
                } else {
                    output.to_string()
                }
            }
            _ => {
                // Generic truncation: Head + Tail, ensuring UTF-8 boundaries.
                let head_byte_len = 1000;
                let tail_byte_len = 500;

                let mut head_end = head_byte_len.min(output.len());
                while !output.is_char_boundary(head_end) && head_end > 0 {
                    head_end -= 1;
                }
                let start = &output[..head_end];

                let mut tail_start = output.len().saturating_sub(tail_byte_len);
                while !output.is_char_boundary(tail_start) && tail_start < output.len() {
                    tail_start += 1;
                }
                let end = &output[tail_start..];

                let truncated_len = output.len().saturating_sub(start.len() + end.len());

                format!(
                    "{}\n... [{} bytes truncated] ...\n{}",
                    start,
                    truncated_len,
                    end
                )
            }
        }
    }

    /// Extracts a preview of a string (first N words).
    ///
    /// This helper extracts the first N words from a string for logging.
    ///
    /// # Arguments
    /// * `s` - String to preview
    /// * `word_count` - Number of words to extract
    ///
    /// # Returns
    /// String preview with "..." appended if truncated
    fn extract_string_preview(s: &str, word_count: usize) -> String {
        let words: Vec<&str> = s.split_whitespace().take(word_count).collect();
        let preview = words.join(" ");
        if s.split_whitespace().count() > word_count {
            format!("{}...", preview)
        } else {
            preview
        }
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
                mode: "plan".to_string(),
                plan_file: None,
            });

        // 2. Update the model field
        agent_config.model = new_model.clone();

        // 3. Create new version with updated agent_config
        let new_app_data = serde_json::to_value(agent_config).map_err(Error::Json)?;

        let new_version = queries::files::create_version(conn, crate::models::files::NewFileVersion {
            id: None,
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

    /// Updates chat metadata and syncs to YAML frontmatter in the file.
    ///
    /// This method updates the AgentConfig in the database (source of truth)
    /// and also writes YAML frontmatter to the chat file for display/debugging.
    pub async fn update_chat_metadata(
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        chat_file_id: Uuid,
        mode: String,
        plan_file: Option<String>,
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
                mode: "plan".to_string(),
                plan_file: None,
            });

        // 2. Update the mode and plan_file fields
        agent_config.mode = mode.clone();
        agent_config.plan_file = plan_file.clone();

        // 3. Create new version with updated agent_config
        let new_app_data = serde_json::to_value(agent_config.clone()).map_err(Error::Json)?;

        let new_version = queries::files::create_version(conn, crate::models::files::NewFileVersion {
            id: None,
            file_id: chat_file_id,
            workspace_id,
            branch: "main".to_string(),
            app_data: new_app_data,
            hash: "metadata-update".to_string(),
            author_id: None,
        }).await?;

        queries::files::update_latest_version_id(conn, chat_file_id, new_version.id).await?;

        // 4. Sync YAML frontmatter to file
        Self::sync_yaml_frontmatter(conn, storage, workspace_id, chat_file_id, &agent_config).await?;

        tracing::info!(
            "[ChatService] Updated metadata for chat {}: mode={}, plan_file={:?}",
            chat_file_id,
            mode,
            plan_file
        );

        Ok(())
    }

    /// Syncs chat metadata to YAML frontmatter in the file.
    ///
    /// Reads the current file content, wraps it with YAML frontmatter,
    /// and writes it back. This keeps the file in sync with database metadata.
    async fn sync_yaml_frontmatter(
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        chat_file_id: Uuid,
        agent_config: &AgentConfig,
    ) -> Result<()> {
        // 1. Get the file path
        let file = queries::files::get_file_by_id(conn, chat_file_id).await?;

        // 2. Read current file content
        let current_content = if let Ok(content) = storage.read_file(workspace_id, &file.path).await {
            String::from_utf8(content).unwrap_or_default()
        } else {
            String::new()
        };

        // 3. Parse existing content to separate frontmatter from body
        let parsed = YamlFrontmatter::parse(&current_content);
        let body_content = if let Ok(parsed) = parsed {
            parsed.content
        } else {
            current_content.clone()
        };

        // 4. Create new frontmatter with updated metadata
        let frontmatter = ChatFrontmatter::from_agent_config(agent_config);
        let yaml_frontmatter = YamlFrontmatter::new(frontmatter, body_content);

        // 5. Serialize with YAML frontmatter
        let new_content = yaml_frontmatter.serialize()?;

        // 6. Write back to file (convert to bytes)
        storage.write_latest_file(workspace_id, &file.path, new_content.as_bytes()).await?;

        // 7. Touch file to update timestamp
        queries::files::touch_file(conn, chat_file_id).await?;

        tracing::debug!("[ChatService] Synced YAML frontmatter for chat {}", chat_file_id);

        Ok(())
    }

    /// Parses YAML frontmatter from a chat file for debugging/display purposes.
    ///
    /// This method reads the chat file and extracts the YAML frontmatter
    /// for display or debugging. Returns None if the file doesn't exist
    /// or has no frontmatter.
    pub async fn get_yaml_frontmatter(
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        chat_file_id: Uuid,
    ) -> Result<Option<ChatFrontmatter>> {
        // 1. Get the file path
        let file = queries::files::get_file_by_id(conn, chat_file_id).await?;

        // 2. Read file content
        let content = match storage.read_file(workspace_id, &file.path).await {
            Ok(bytes) => String::from_utf8(bytes).unwrap_or_default(),
            Err(_) => return Ok(None),
        };

        // 3. Parse YAML frontmatter
        let parsed = YamlFrontmatter::parse(&content)?;

        Ok(Some(parsed.frontmatter))
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
        let mut agent_config = if let Some(_version_id) = file.latest_version_id {
            if let Ok(version) = queries::files::get_latest_version(conn, chat_file_id).await {
                serde_json::from_value(version.app_data).unwrap_or_else(|_| crate::models::chat::AgentConfig {
                    agent_id: None,
                    model: DEFAULT_CHAT_MODEL.to_string(),
                    temperature: 0.7,
                    persona_override: None,
                    previous_response_id: None,
                    mode: "plan".to_string(),
                    plan_file: None,
                })
            } else {
                 crate::models::chat::AgentConfig {
                    agent_id: None,
                    model: DEFAULT_CHAT_MODEL.to_string(),
                    temperature: 0.7,
                    persona_override: None,
                    previous_response_id: None,
                    mode: "plan".to_string(),
                    plan_file: None,
                }
            }
        } else {
             crate::models::chat::AgentConfig {
                agent_id: None,
                model: DEFAULT_CHAT_MODEL.to_string(),
                temperature: 0.7,
                persona_override: None,
                previous_response_id: None,
                mode: "plan".to_string(),
                plan_file: None,
            }
        };

        // Runtime migration: Convert legacy model strings to new format
        // Detects legacy format (no colon) and adds "openai:" prefix
        if !agent_config.model.contains(':') {
            tracing::warn!(
                chat_file_id = %chat_file_id,
                legacy_model = %agent_config.model,
                "Migrating legacy model format to new provider:model format"
            );
            agent_config.model = format!("openai:{}", agent_config.model);
        }

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
