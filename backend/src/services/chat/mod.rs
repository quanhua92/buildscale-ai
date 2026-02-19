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
    build_sorted_context_items, filter_messages_for_context, get_indices_to_truncate,
    get_old_tool_result_indices, messages_to_context_items, truncate_at_char_boundary,
    truncate_tool_output, AttachmentManager, AttachmentKey, AttachmentValue, ContextItem,
    ESTIMATED_CHARS_PER_TOKEN, KEEP_RECENT_TOOL_RESULTS, HistoryManager, PRIORITY_ESSENTIAL,
    PRIORITY_HIGH, PRIORITY_LOW, PRIORITY_MEDIUM, TRUNCATED_TOOL_RESULT_PREVIEW,
};

pub use sync::{ChatFrontmatter, YamlFrontmatter};

#[cfg(test)]
mod tests;

use crate::{
    error::{Error, Result},
    models::chat::{AgentConfig, ChatAttachment, ChatMessage, ChatMessageMetadata, ChatMessageRole, NewChatMessage, DEFAULT_CHAT_MODEL},
    models::requests::{GrepResult, GlobResult, LsResult},
    queries, DbConn,
};
use uuid::Uuid;

/// Default token limit for the context window (128k for modern models).
pub const DEFAULT_CONTEXT_TOKEN_LIMIT: usize = 128000;

/// Max length for tool output before truncation (1MB)
const MAX_TOOL_OUTPUT_LENGTH: usize = 1_048_576;
/// Max length for 'write' tool content arg (10MB)
const MAX_WRITE_CONTENT_LENGTH: usize = 10_485_760;
/// Max length for 'edit' tool diff args (1MB)
const MAX_EDIT_DIFF_LENGTH: usize = 1_048_576;
/// Number of items to preview for 'ls' tool
const LS_PREVIEW_ITEMS: usize = 50;
/// Number of matches to preview for 'grep' tool
const GREP_PREVIEW_MATCHES: usize = 20;
/// Number of lines to preview for content truncation
const CONTENT_PREVIEW_LINES: usize = 20;

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
                // Truncate 'content' field (can be very large)
                if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                    if content.len() > MAX_WRITE_CONTENT_LENGTH {
                        // Use line-based preview for better readability (like read tool)
                        let lines: Vec<&str> = content.lines().collect();
                        let preview_lines: Vec<&str> = lines.iter().take(CONTENT_PREVIEW_LINES).cloned().collect();
                        let preview = preview_lines.join("\n");
                        obj.insert(
                            "content".to_string(),
                            serde_json::json!(format!("{}\n... [truncated, {} lines total]", preview, lines.len())),
                        );
                    }
                }
            }
            "edit" => {
                // Truncate 'old_string', 'new_string', and 'insert_content' fields
                for field in ["old_string", "new_string", "insert_content"] {
                    if let Some(val) = obj.get(field).and_then(|v| v.as_str()) {
                        if val.len() > MAX_EDIT_DIFF_LENGTH {
                            // Use line-based preview for better readability
                            let lines: Vec<&str> = val.lines().collect();
                            let preview_lines: Vec<&str> = lines.iter().take(CONTENT_PREVIEW_LINES).cloned().collect();
                            let preview = preview_lines.join("\n");
                            obj.insert(
                                field.to_string(),
                                serde_json::json!(format!("{}\n... [truncated, {} lines]", preview, lines.len())),
                            );
                        }
                    }
                }
            }
            // Tools that don't need input truncation (arguments are small):
            "ls" => {
                // No truncation needed - arguments are small (path, recursive flag)
            }
            "read" => {
                // No truncation needed - only path argument (plus optional offset/limit)
            }
            "rm" => {
                // No truncation needed - only path argument
            }
            "mv" => {
                // No truncation needed - two path arguments (small)
            }
            "touch" => {
                // No truncation needed - only path argument
            }
            "grep" => {
                // No truncation needed - pattern, path, and context params are small
            }
            "mkdir" => {
                // No truncation needed - only path argument
            }
            "ask_user" => {
                // No truncation needed - question text is typically short
                // TODO: Consider truncation if questions become very long
            }
            "exit_plan_mode" => {
                // No truncation needed - only path argument
            }
            "glob" => {
                // No truncation needed - pattern and path arguments are small
            }
            "file_info" => {
                // No truncation needed - only path argument
            }
            "read_multiple_files" => {
                // No truncation needed for paths array (small strings)
                // Limit parameter is a small integer
            }
            "find" => {
                // No truncation needed - all arguments are small strings/ints
            }
            "cat" => {
                // No truncation needed for paths array and boolean flags
            }
            unknown_tool => {
                // ERROR-level: Unknown tool can cause database bloat
                // Tool was added to ToolExecutor but truncate logic not added here
                tracing::error!(
                    tool = %unknown_tool,
                    "Unknown tool '{}' encountered in summarize_tool_inputs. \
                     Tool was added to ToolExecutor but truncate logic not added. \
                     This may cause DATABASE BLOAT if tool has large inputs. \
                     Add explicit handling for this tool.",
                    unknown_tool
                );
            }
        }

        summary
    }

    /// Summarizes tool outputs (results) to prevent database bloat.
    /// Uses smart semantic truncation for grep/glob/ls to preserve JSON structure.
    pub fn summarize_tool_outputs(tool_name: &str, output: &str) -> String {
        // If output is small, keep it all
        if output.len() < MAX_TOOL_OUTPUT_LENGTH {
            return output.to_string();
        }

        match tool_name {
            "read" => {
                // INDUSTRY STANDARD: Parse → Truncate content → Re-serialize
                Self::truncate_read_result(output)
            }
            "cat" => {
                // INDUSTRY STANDARD: Parse → Truncate content → Re-serialize
                Self::truncate_cat_result(output)
            }
            "read_multiple_files" => {
                // INDUSTRY STANDARD: Parse → Truncate content → Re-serialize
                Self::truncate_read_multiple_files_result(output)
            }
            "grep" => {
                // INDUSTRY STANDARD: Parse → Truncate → Re-serialize
                // Preserves JSON structure and complete match records with context
                Self::truncate_grep_result(output)
            }
            "glob" => {
                // INDUSTRY STANDARD: Parse → Truncate → Re-serialize
                Self::truncate_glob_result(output)
            }
            "ls" => {
                // INDUSTRY STANDARD: Parse → Truncate → Re-serialize
                Self::truncate_ls_result(output)
            }
            "find" => {
                // INDUSTRY STANDARD: Parse → Truncate → Re-serialize
                // Find returns {matches: [...]} like glob, use same truncation
                Self::truncate_find_result(output)
            }
            other_tool => {
                // WARN-level: Generic truncation works but might be suboptimal
                // Most tool outputs are short (error messages, confirmation text)
                // Long outputs get head+tail preview with UTF-8 boundary safety
                // If a tool needs special handling, add explicit case above
                tracing::warn!(
                    tool = %other_tool,
                    "Tool '{}' using generic output truncation. If this tool needs \
                     special handling (like read/ls/grep/glob), add explicit case.",
                    other_tool
                );
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

    /// Fallback line-based truncation for malformed JSON.
    /// Used when JSON parsing fails in smart truncation functions.
    fn fallback_line_truncation(output: &str, limit: usize, item_name: &str) -> String {
        let lines: Vec<&str> = output.lines().collect();
        if lines.len() > limit {
            let preview = lines.iter().take(limit).cloned().collect::<Vec<_>>().join("\n");
            format!("{}\n... ({} more {})", preview, lines.len() - limit, item_name)
        } else {
            output.to_string()
        }
    }

    /// Generic truncation for list-based result types.
    /// Uses the TruncatableList trait to truncate lists while preserving JSON structure.
    fn truncate_list_result<T>(
        output: &str,
        limit: usize,
        tool_name: &'static str,
        item_name: &str,
    ) -> String
    where
        T: serde::de::DeserializeOwned + serde::Serialize + crate::models::requests::TruncatableList,
    {
        if let Ok(parsed) = serde_json::from_str::<T>(output) {
            if parsed.list_len() > limit {
                let truncated = parsed.truncate_list(limit);
                if let Ok(compact) = serde_json::to_string(&truncated) {
                    return compact;
                }
            }
            output.to_string()
        } else {
            tracing::warn!(
                tool = tool_name,
                output_len = output.len(),
                "Failed to parse result as JSON, falling back to line-based truncation"
            );
            Self::fallback_line_truncation(output, limit, item_name)
        }
    }

    /// Smart truncation for grep results: Parse → Truncate at match boundaries → Re-serialize
    /// Preserves JSON structure and complete match records with context.
    fn truncate_grep_result(output: &str) -> String {
        Self::truncate_list_result::<GrepResult>(output, GREP_PREVIEW_MATCHES, "grep", "matches")
    }

    /// Smart truncation for glob results: Parse → Truncate at match boundaries → Re-serialize
    fn truncate_glob_result(output: &str) -> String {
        Self::truncate_list_result::<GlobResult>(output, LS_PREVIEW_ITEMS, "glob", "matches")
    }

    /// Smart truncation for find results: Parse → Truncate at match boundaries → Re-serialize
    fn truncate_find_result(output: &str) -> String {
        use crate::models::requests::FindResult;
        Self::truncate_list_result::<FindResult>(output, LS_PREVIEW_ITEMS, "find", "matches")
    }

    /// Smart truncation for ls results: Parse → Truncate at entry boundaries → Re-serialize
    fn truncate_ls_result(output: &str) -> String {
        Self::truncate_list_result::<LsResult>(output, LS_PREVIEW_ITEMS, "ls", "items")
    }

    /// Smart truncation for read results: Parse → Truncate content → Re-serialize
    fn truncate_read_result(output: &str) -> String {
        use crate::models::requests::ReadResult;
        if let Ok(mut parsed) = serde_json::from_str::<ReadResult>(output) {
            if let Some(content_str) = parsed.content.as_str() {
                if content_str.len() > MAX_TOOL_OUTPUT_LENGTH {
                    let preview_lines: Vec<&str> = content_str.lines().take(CONTENT_PREVIEW_LINES).collect();
                    let preview = preview_lines.join("\n");
                    let line_count = content_str.lines().count();
                    parsed.content = serde_json::json!(
                        format!("[{} lines total, showing first {}]\n{}\n... [content truncated]",
                            line_count, CONTENT_PREVIEW_LINES, preview)
                    );
                    if let Ok(truncated) = serde_json::to_string(&parsed) {
                        return truncated;
                    }
                }
            }
            output.to_string()
        } else {
            tracing::warn!(
                tool = "read",
                output_len = output.len(),
                "Failed to parse read result as JSON, falling back to line-based truncation"
            );
            Self::fallback_line_truncation(output, CONTENT_PREVIEW_LINES, "lines")
        }
    }

    /// Smart truncation for cat results: Parse → Truncate content → Re-serialize
    fn truncate_cat_result(output: &str) -> String {
        use crate::models::requests::CatResult;
        if let Ok(mut parsed) = serde_json::from_str::<CatResult>(output) {
            if parsed.content.len() > MAX_TOOL_OUTPUT_LENGTH {
                let preview_lines: Vec<&str> = parsed.content.lines().take(CONTENT_PREVIEW_LINES).collect();
                let preview = preview_lines.join("\n");
                let line_count = parsed.content.lines().count();
                parsed.content = format!(
                    "[{} lines total, showing first {}]\n{}\n... [content truncated]",
                    line_count, CONTENT_PREVIEW_LINES, preview
                );
                if let Ok(truncated) = serde_json::to_string(&parsed) {
                    return truncated;
                }
            }
            output.to_string()
        } else {
            tracing::warn!(
                tool = "cat",
                output_len = output.len(),
                "Failed to parse cat result as JSON, falling back to line-based truncation"
            );
            Self::fallback_line_truncation(output, CONTENT_PREVIEW_LINES, "lines")
        }
    }

    /// Smart truncation for read_multiple_files results: Parse → Truncate each file's content → Re-serialize
    fn truncate_read_multiple_files_result(output: &str) -> String {
        use crate::models::requests::ReadMultipleFilesResult;
        if let Ok(mut parsed) = serde_json::from_str::<ReadMultipleFilesResult>(output) {
            let mut any_truncated = false;
            for file in &mut parsed.files {
                if let Some(content_str) = file.content.as_ref().and_then(|c| c.as_str()) {
                    if content_str.len() > MAX_TOOL_OUTPUT_LENGTH / 2 {
                        let preview_lines: Vec<&str> = content_str.lines().take(CONTENT_PREVIEW_LINES).collect();
                        let preview = preview_lines.join("\n");
                        let line_count = content_str.lines().count();
                        file.content = Some(serde_json::json!(
                            format!("[{} lines, showing first {}]\n{}\n... [truncated]",
                                line_count, CONTENT_PREVIEW_LINES, preview)
                        ));
                        any_truncated = true;
                    }
                }
            }
            if any_truncated {
                if let Ok(truncated) = serde_json::to_string(&parsed) {
                    return truncated;
                }
            }
            output.to_string()
        } else {
            tracing::warn!(
                tool = "read_multiple_files",
                output_len = output.len(),
                "Failed to parse read_multiple_files result as JSON, falling back to line-based truncation"
            );
            Self::fallback_line_truncation(output, LS_PREVIEW_ITEMS, "lines")
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
    ///
    /// # Arguments
    /// * `exclude_last_message` - If true, excludes the last message (used for AI context where
    ///   last message is the user's prompt). If false, includes all messages (used for Context UI).
    pub async fn build_context(
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        chat_file_id: Uuid,
        default_persona: &str,
        default_context_token_limit: usize,
        exclude_last_message: bool,
    ) -> Result<BuiltContext> {
        // 1. Load Session Identity & History
        let messages = queries::chat::get_messages_by_file_id(conn, workspace_id, chat_file_id).await?;

        // 2. Hydrate Persona
        let persona = default_persona.to_string();

        // 3. Extract history (optionally exclude last message which is the prompt for AI context)
        let history_messages = if exclude_last_message && messages.len() > 1 {
            messages[..messages.len() - 1].to_vec()
        } else if exclude_last_message {
            Vec::new()
        } else {
            messages.clone()
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

                        // Get the file's updated_at timestamp for cache optimization
                        let source_modified_at = Some(file_with_content.file.updated_at);

                        // Add to attachment manager with workspace file key
                        // Use MEDIUM priority for user-attached files
                        attachment_manager.add_fragment(
                            AttachmentKey::WorkspaceFile(*file_id),
                            AttachmentValue {
                                content,
                                priority: PRIORITY_MEDIUM,
                                tokens: estimated_tokens,
                                is_essential: false,
                                created_at: chrono::Utc::now(),
                                updated_at: source_modified_at,
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

    // ========================================================================
    // Context API Methods
    // ========================================================================

    /// Content preview length for context API responses
    const CONTENT_PREVIEW_LENGTH: usize = 200;

    /// Get detailed context information for debugging/inspection.
    ///
    /// Returns structured information about everything sent to the AI,
    /// including system prompt, history, tools, and attachments with
    /// character counts, token estimates, and helpful statistics.
    pub async fn get_context_info(
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        chat_file_id: Uuid,
        default_persona: &str,
        fallback_token_limit: usize,
    ) -> Result<crate::models::chat::ChatContextResponse> {
        // 1. Get session for model/mode info first (needed to determine token limit)
        let _file = queries::files::get_file_by_id(conn, chat_file_id).await?;

        let agent_config: crate::models::chat::AgentConfig = match queries::files::get_latest_version(conn, chat_file_id).await {
            Ok(version) => serde_json::from_value(version.app_data)?,
            Err(_) => {
                return Err(Error::NotFound("Chat has no configuration".into()));
            }
        };

        // 2. Look up model's context window from database
        let token_limit = Self::get_model_context_window(conn, &agent_config.model)
            .await
            .unwrap_or(fallback_token_limit);

        // 3. Build context - include ALL messages for Context UI (no exclusion)
        let context = Self::build_context(
            conn, storage, workspace_id, chat_file_id, default_persona, token_limit,
            false, // exclude_last_message=false for Context UI to show all messages
        ).await?;

        // 4. Build each section
        let system_prompt = Self::build_system_prompt_section(&context.persona, &agent_config.mode);
        let history = Self::build_history_section(&context.history.messages, &context.attachment_manager);
        let tools = Self::build_tools_section();
        let attachments = Self::build_attachments_section(&context.attachment_manager);

        // 5. Build summary
        let summary = Self::build_context_summary(
            &system_prompt, &history, &tools, &attachments, &agent_config.model, token_limit
        );

        Ok(crate::models::chat::ChatContextResponse {
            system_prompt,
            history,
            tools,
            attachments,
            summary,
        })
    }

    /// Get the context window for a model from the database.
    /// Model string format: "provider:model_name" (e.g., "openai:gpt-4o")
    async fn get_model_context_window(conn: &mut DbConn, model: &str) -> Option<usize> {
        let (provider, model_name) = model.split_once(':')?;
        let ai_model = queries::ai_models::get_model_by_provider_and_name_conn(
            conn, provider, model_name
        ).await.ok()??;
        ai_model.context_window.map(|cw| cw as usize)
    }

    fn build_system_prompt_section(persona: &str, mode: &str) -> crate::models::chat::SystemPromptSection {
        let persona_type = if persona.contains("Planner") { "planner" }
            else if persona.contains("Builder") { "builder" }
            else { "assistant" };

        crate::models::chat::SystemPromptSection {
            content: persona.to_string(),
            char_count: persona.len(),
            token_count: persona.len() / ESTIMATED_CHARS_PER_TOKEN,
            persona_type: persona_type.to_string(),
            mode: mode.to_string(),
        }
    }

    fn build_history_section(
        messages: &[ChatMessage],
        attachment_manager: &AttachmentManager,
    ) -> crate::models::chat::HistorySection {
        // Use centralized context building from context.rs (single source of truth)
        // Pass None for render_fn since Context UI doesn't need rendered attachments
        let items = build_sorted_context_items::<fn(&AttachmentKey, &AttachmentValue) -> String>(
            messages,
            Some(attachment_manager),
            None,
        );

        // Use centralized truncation logic
        let indices_to_truncate = get_indices_to_truncate(&items);

        // Map back to message IDs that should be truncated
        let ids_to_truncate: std::collections::HashSet<Uuid> = items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if indices_to_truncate.contains(&i) {
                    if let ContextItem::Message { metadata, .. } = item {
                        if metadata.message_type.as_deref() == Some("tool_result") {
                            // Find the original message to get its ID
                            return messages
                                .iter()
                                .find(|msg| msg.metadata.message_type.as_deref() == Some("tool_result")
                                    && msg.content == match item {
                                        ContextItem::Message { content, .. } => content,
                                        _ => "",
                                    })
                                .map(|msg| msg.id);
                        }
                    }
                }
                None
            })
            .collect();

        // Build history messages with truncation applied (using centralized filtering)
        let filtered_messages = filter_messages_for_context(messages);
        let history_messages: Vec<crate::models::chat::HistoryMessageInfo> = filtered_messages
            .iter()
            .map(|msg| {
                let mut content = msg.content.clone();
                let original_length = msg.content.len();

                // Truncate old tool results using shared function
                if ids_to_truncate.contains(&msg.id) {
                    content = truncate_tool_output(&content);
                }

                // Calculate token count from truncated content (not original)
                let truncated_length = content.len();

                let preview = if content.len() > Self::CONTENT_PREVIEW_LENGTH {
                    let truncate_at = truncate_at_char_boundary(&content, Self::CONTENT_PREVIEW_LENGTH);
                    format!("{}...", &content[..truncate_at])
                } else {
                    content
                };

                crate::models::chat::HistoryMessageInfo {
                    role: msg.role.to_string().to_lowercase(),
                    content_preview: preview,
                    content_length: original_length,
                    token_count: truncated_length / ESTIMATED_CHARS_PER_TOKEN,
                    metadata: Some(crate::models::chat::HistoryMessageMetadata {
                        message_type: msg.metadata.message_type.clone(),
                        reasoning_id: msg.metadata.reasoning_id.clone(),
                        tool_name: msg.metadata.tool_name.clone(),
                        model: msg.metadata.model.clone(),
                    }),
                    created_at: msg.created_at,
                }
            })
            .collect();

        crate::models::chat::HistorySection {
            message_count: history_messages.len(),
            total_tokens: history_messages.iter().map(|m| m.token_count).sum(),
            messages: history_messages,
        }
    }

    fn build_tools_section() -> crate::models::chat::ToolsSection {
        // Get all tool definitions
        let tools = crate::tools::get_all_tool_definitions();
        let schema_json = serde_json::to_string(&tools).unwrap_or_default();
        let tool_count = tools.len();

        crate::models::chat::ToolsSection {
            tools,
            tool_count,
            estimated_schema_tokens: schema_json.len() / ESTIMATED_CHARS_PER_TOKEN,
        }
    }

    fn build_attachments_section(manager: &AttachmentManager) -> crate::models::chat::AttachmentsSection {
        let attachments: Vec<crate::models::chat::AttachmentInfo> = manager.map.iter().map(|(key, value)| {
            let (attachment_type, id) = match key {
                AttachmentKey::WorkspaceFile(fid) => ("workspace_file", *fid),
                AttachmentKey::ActiveSkill(sid) => ("skill", *sid),
                AttachmentKey::SystemPersona => ("system_persona", Uuid::nil()),
                AttachmentKey::Environment => ("environment", Uuid::nil()),
                AttachmentKey::ChatHistory => ("chat_history", Uuid::nil()),
                AttachmentKey::UserRequest => ("user_request", Uuid::nil()),
            };

            let preview = if value.content.len() > Self::CONTENT_PREVIEW_LENGTH {
                format!("{}...", &value.content[..Self::CONTENT_PREVIEW_LENGTH])
            } else {
                value.content.clone()
            };

            crate::models::chat::AttachmentInfo {
                attachment_type: attachment_type.to_string(),
                id,
                content_preview: preview,
                content_length: value.content.len(),
                token_count: value.tokens,
                priority: value.priority,
                is_essential: value.is_essential,
                created_at: value.created_at,
                updated_at: value.updated_at,
            }
        }).collect();

        crate::models::chat::AttachmentsSection {
            attachment_count: attachments.len(),
            total_tokens: attachments.iter().map(|a| a.token_count).sum(),
            attachments,
        }
    }

    fn build_context_summary(
        system_prompt: &crate::models::chat::SystemPromptSection,
        history: &crate::models::chat::HistorySection,
        tools: &crate::models::chat::ToolsSection,
        attachments: &crate::models::chat::AttachmentsSection,
        model: &str,
        token_limit: usize,
    ) -> crate::models::chat::ContextSummary {
        let breakdown = crate::models::chat::TokenBreakdown {
            system_prompt_tokens: system_prompt.token_count,
            history_tokens: history.total_tokens,
            tools_tokens: tools.estimated_schema_tokens,
            attachments_tokens: attachments.total_tokens,
        };

        let total = breakdown.system_prompt_tokens + breakdown.history_tokens
            + breakdown.tools_tokens + breakdown.attachments_tokens;

        let utilization = if token_limit > 0 {
            (total as f64 / token_limit as f64) * 100.0
        } else { 0.0 };

        crate::models::chat::ContextSummary {
            total_tokens: total,
            utilization_percent: utilization,
            model: model.to_string(),
            token_limit,
            breakdown,
        }
    }

    /// Generates a chat name from message content.
    /// Uses smart truncation to avoid cutting words in half.
    pub fn generate_chat_name(content: &str, max_length: usize) -> String {
        const PREFIX: &str = "Chat: ";

        // Trim whitespace first
        let content = content.trim();

        // If content is empty, return default
        if content.is_empty() {
            return "Chat".to_string();
        }

        // Adjust max_length to account for prefix
        let adjusted_max_length = max_length.saturating_sub(PREFIX.len());

        // If content fits within adjusted max_length, use it all
        if content.len() <= adjusted_max_length {
            return format!("{}{}", PREFIX, content);
        }

        // Find safe truncation point (don't cut words in half)
        let snippet_end = content.char_indices()
            .nth(adjusted_max_length)
            .map_or(content.len(), |(idx, _)| idx);

        // If we're cutting mid-word, find the last space
        let safe_end = if snippet_end < content.len() {
            content[..snippet_end]
                .rfind(' ')
                .unwrap_or(snippet_end)
        } else {
            snippet_end
        };

        let truncated = &content[..safe_end];
        format!("{}{}", PREFIX, truncated)
    }

    /// Updates the chat file name based on recent message content.
    pub async fn update_chat_name(
        conn: &mut DbConn,
        chat_file_id: Uuid,
        new_name: String,
    ) -> Result<()> {
        // 1. Get current file info
        let current_file = queries::files::get_file_by_id(conn, chat_file_id).await?;

        // 2. Generate new slug and path from name (keep same pattern as creation)
        let new_slug = format!("chat-{}.chat", chat_file_id);
        let new_path = format!("/chats/{}", new_slug);

        // 3. Update file metadata (name, slug, path)
        queries::files::update_file_metadata(
            conn,
            chat_file_id,
            current_file.parent_id,
            &new_name,
            &new_slug,
            &new_path,
            current_file.is_virtual,
            current_file.is_remote,
            current_file.permission,
        ).await?;

        tracing::info!("[ChatService] Updated chat name for {} to {}", chat_file_id, new_name);

        Ok(())
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
