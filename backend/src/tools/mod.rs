//! Tool system for BuildScale
//!
//! This module provides an extensible toolset that operates on files in workspaces.
//! Tools follow the "Everything is a File" philosophy, providing filesystem-like
//! operations (ls, read, write, rm, mv, touch) backed by the database.
//!
//! ## Module Organization
//!
//! The tools are organized into subdirectories by category:
//!
//! - `file/` - File system tools (ls, read, write, edit, rm, mv, touch, mkdir, grep, glob, find, cat, file_info, read_multiple_files)
//! - `memory/` - Memory tools (memory_set, memory_get, memory_search, memory_delete, memory_list)
//! - `plan/` - Plan mode tools (ask_user, exit_plan_mode, plan_write, plan_read, plan_edit, plan_list)
//! - `web/` - Web tools (web_fetch, web_search)

// Subdirectory modules (layered structure)
pub mod file;
pub mod memory;
pub mod plan;
pub mod web;

// Helpers stay at root level
pub mod helpers;

use crate::{DbConn, error::{Error, Result}, models::requests::ToolResponse, models::chat::ToolDefinition, services::storage::FileStorageService, state::TagIndexMessage, state::LinkIndexMessage};
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Error message shown when tools are restricted in Plan Mode
///
/// This constant provides a consistent, helpful error message across all tools
/// that are restricted in Plan Mode, explaining how to transition to Build Mode.
pub const PLAN_MODE_ERROR: &str = "System is in Plan Mode. To switch to Build Mode: 1) Use ask_user with Accept/Reject buttons to request approval, 2) When user clicks Accept (you'll see [Answered: \"Accept\"]), call exit_plan_mode with your plan file path.";

/// Tool configuration for execution context
///
/// This configuration object is passed to all tools during execution
/// and provides context about the current execution mode and state.
///
/// # Examples
///
/// ```rust
/// use buildscale::tools::ToolConfig;
///
/// let config = ToolConfig {
///     plan_mode: true,
///     active_plan_path: Some("/plans/project-roadmap.plan".to_string()),
///     chat_id: None,
///     tag_index_tx: None,
///     link_index_tx: None,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ToolConfig {
    /// Whether the system is in Plan Mode (true) or Build Mode (false)
    ///
    /// In Plan Mode, file modification tools are restricted to .plan files only.
    /// In Build Mode, all tools have full access.
    pub plan_mode: bool,

    /// Path to the active plan file (only set in Build Mode)
    ///
    /// This is the absolute path to the .plan file that was approved and
    /// is now being executed. The plan content is injected into the Builder
    /// agent's context.
    pub active_plan_path: Option<String>,

    /// Chat ID for tools that need to update chat metadata
    ///
    /// Used by exit_plan_mode to update the chat file's mode.
    pub chat_id: Option<Uuid>,

    /// Channel to signal tag indexer worker when files are modified
    ///
    /// Used by write/edit tools to trigger tag reindexing for markdown files.
    pub tag_index_tx: Option<mpsc::UnboundedSender<TagIndexMessage>>,

    /// Channel to signal link indexer worker when files are modified
    ///
    /// Used by write/edit tools to trigger link reindexing for markdown files.
    pub link_index_tx: Option<mpsc::UnboundedSender<LinkIndexMessage>>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            plan_mode: false, // Default to Build Mode for normal operation
            active_plan_path: None,
            chat_id: None,
            tag_index_tx: None,
            link_index_tx: None,
        }
    }
}

/// Tool trait for extensible toolset
///
/// All tools implement this trait to provide a unified execution interface.
/// This allows for easy addition of new tools to the system.
///
/// # Breaking Change Notice
/// As of Plan Mode implementation, all tools must accept a `ToolConfig` parameter
/// in their execute method. This provides context about the current execution mode
/// (Plan vs Build) and allows tools to enforce mode-specific restrictions.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the name of this tool
    fn name(&self) -> &'static str;

    /// Returns a description of what this tool does
    fn description(&self) -> &'static str;

    /// Returns the JSON schema definition for this tool's arguments
    fn definition(&self) -> Value;

    /// Executes the tool with given arguments
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `storage` - File storage service
    /// * `workspace_id` - ID of workspace to operate on
    /// * `user_id` - ID of authenticated user executing the tool
    /// * `config` - Tool execution configuration (Plan/Build mode context)
    /// * `args` - Tool-specific arguments as JSON value
    ///
    /// # Returns
    /// Tool response with success status and result or error
    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse>;
}

/// Get tool by name from registry
///
/// Returns a closure that executes the tool when called with the appropriate arguments
///
/// # Arguments
/// * `tool_name` - Name of the tool to retrieve (e.g., "ls", "read", "write", "rm", "mv", "touch")
///
/// # Returns
/// * `Ok(ToolExecutor)` - The tool executor closure
/// * `Err(Error)` - If tool name is not found
pub fn get_tool_executor(tool_name: &str) -> Result<ToolExecutor> {
    match tool_name {
        // File tools
        "ls" => Ok(ToolExecutor::Ls),
        "read" => Ok(ToolExecutor::Read),
        "write" => Ok(ToolExecutor::Write),
        "rm" => Ok(ToolExecutor::Rm),
        "mv" => Ok(ToolExecutor::Mv),
        "touch" => Ok(ToolExecutor::Touch),
        "edit" => Ok(ToolExecutor::Edit),
        "grep" => Ok(ToolExecutor::Grep),
        "mkdir" => Ok(ToolExecutor::Mkdir),
        "glob" => Ok(ToolExecutor::Glob),
        "file_info" => Ok(ToolExecutor::FileInfo),
        "read_multiple_files" => Ok(ToolExecutor::ReadMultipleFiles),
        "find" => Ok(ToolExecutor::Find),
        "cat" => Ok(ToolExecutor::Cat),
        // Plan tools
        "ask_user" => Ok(ToolExecutor::AskUser),
        "exit_plan_mode" => Ok(ToolExecutor::ExitPlanMode),
        "plan_write" => Ok(ToolExecutor::PlanWrite),
        "plan_read" => Ok(ToolExecutor::PlanRead),
        "plan_edit" => Ok(ToolExecutor::PlanEdit),
        "plan_list" => Ok(ToolExecutor::PlanList),
        // Memory tools
        "memory_set" => Ok(ToolExecutor::MemorySet),
        "memory_get" => Ok(ToolExecutor::MemoryGet),
        "memory_search" => Ok(ToolExecutor::MemorySearch),
        "memory_delete" => Ok(ToolExecutor::MemoryDelete),
        "memory_list" => Ok(ToolExecutor::MemoryList),
        // Web tools
        "web_fetch" => Ok(ToolExecutor::WebFetch),
        "web_search" => Ok(ToolExecutor::WebSearch),
        _ => Err(Error::NotFound(format!("Tool '{}' not found", tool_name))),
    }
}

/// Normalizes a file system path for consistency.
/// Trims whitespace, ensures it starts with a / and has no trailing /.
/// Collapses multiple consecutive slashes into one and handles . and .. segments.
pub fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return "/".to_string();
    }

    let mut components = Vec::new();
    for segment in trimmed.split('/') {
        match segment {
            "" | "." => continue,
            ".." => {
                components.pop();
            }
            _ => components.push(segment),
        }
    }

    if components.is_empty() {
        return "/".to_string();
    }

    format!("/{}", components.join("/"))
}

/// Tool executor enum for dispatching tool execution
pub enum ToolExecutor {
    // File tools
    Ls,
    Read,
    Write,
    Rm,
    Mv,
    Touch,
    Edit,
    Grep,
    Mkdir,
    Glob,
    FileInfo,
    ReadMultipleFiles,
    Find,
    Cat,
    // Plan tools
    AskUser,
    ExitPlanMode,
    PlanWrite,
    PlanRead,
    PlanEdit,
    PlanList,
    // Memory tools
    MemorySet,
    MemoryGet,
    MemorySearch,
    MemoryDelete,
    MemoryList,
    // Web tools
    WebFetch,
    WebSearch,
}

impl ToolExecutor {
    pub async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let name = match self {
            // File tools
            ToolExecutor::Ls => "ls",
            ToolExecutor::Read => "read",
            ToolExecutor::Write => "write",
            ToolExecutor::Rm => "rm",
            ToolExecutor::Mv => "mv",
            ToolExecutor::Touch => "touch",
            ToolExecutor::Edit => "edit",
            ToolExecutor::Grep => "grep",
            ToolExecutor::Mkdir => "mkdir",
            ToolExecutor::Glob => "glob",
            ToolExecutor::FileInfo => "file_info",
            ToolExecutor::ReadMultipleFiles => "read_multiple_files",
            ToolExecutor::Find => "find",
            ToolExecutor::Cat => "cat",
            // Plan tools
            ToolExecutor::AskUser => "ask_user",
            ToolExecutor::ExitPlanMode => "exit_plan_mode",
            ToolExecutor::PlanWrite => "plan_write",
            ToolExecutor::PlanRead => "plan_read",
            ToolExecutor::PlanEdit => "plan_edit",
            ToolExecutor::PlanList => "plan_list",
            // Memory tools
            ToolExecutor::MemorySet => "memory_set",
            ToolExecutor::MemoryGet => "memory_get",
            ToolExecutor::MemorySearch => "memory_search",
            ToolExecutor::MemoryDelete => "memory_delete",
            ToolExecutor::MemoryList => "memory_list",
            // Web tools
            ToolExecutor::WebFetch => "web_fetch",
            ToolExecutor::WebSearch => "web_search",
        };

        let span = tracing::info_span!("tool_execute", tool = name, workspace_id = %workspace_id, user_id = %user_id);
        let _enter = span.enter();

        tracing::debug!(args = %args, "Tool input");

        let result = match self {
            // File tools
            ToolExecutor::Ls => file::LsTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Read => file::ReadTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Write => file::WriteTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Rm => file::RmTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Mv => file::MvTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Touch => file::TouchTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Edit => file::EditTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Grep => file::GrepTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Mkdir => file::MkdirTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Glob => file::GlobTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::FileInfo => file::FileInfoTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::ReadMultipleFiles => file::ReadMultipleFilesTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Find => file::FindTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Cat => file::CatTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            // Plan tools
            ToolExecutor::AskUser => plan::AskUserTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::ExitPlanMode => plan::ExitPlanModeTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::PlanWrite => plan::PlanWriteTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::PlanRead => plan::PlanReadTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::PlanEdit => plan::PlanEditTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::PlanList => plan::PlanListTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            // Memory tools
            ToolExecutor::MemorySet => memory::MemorySetTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::MemoryGet => memory::MemoryGetTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::MemorySearch => memory::MemorySearchTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::MemoryDelete => memory::MemoryDeleteTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::MemoryList => memory::MemoryListTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            // Web tools
            ToolExecutor::WebFetch => web::WebFetchTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::WebSearch => web::WebSearchTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
        };

        match &result {
            Ok(resp) => {
                if resp.success {
                    tracing::info!("Tool execution successful");
                } else {
                    tracing::error!(
                        error = ?resp.error,
                        "Tool returned logical failure"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    "Tool execution crashed with internal error"
                );
            }
        }

        result
    }
}

/// Get all tool definitions for the context API.
///
/// Returns a list of all available tools with their name, description, and JSON schema parameters.
pub fn get_all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // File tools
        ToolDefinition {
            name: "ls".into(),
            description: file::LsTool.description().into(),
            parameters: file::LsTool.definition(),
        },
        ToolDefinition {
            name: "read".into(),
            description: file::ReadTool.description().into(),
            parameters: file::ReadTool.definition(),
        },
        ToolDefinition {
            name: "write".into(),
            description: file::WriteTool.description().into(),
            parameters: file::WriteTool.definition(),
        },
        ToolDefinition {
            name: "edit".into(),
            description: file::EditTool.description().into(),
            parameters: file::EditTool.definition(),
        },
        ToolDefinition {
            name: "rm".into(),
            description: file::RmTool.description().into(),
            parameters: file::RmTool.definition(),
        },
        ToolDefinition {
            name: "mv".into(),
            description: file::MvTool.description().into(),
            parameters: file::MvTool.definition(),
        },
        ToolDefinition {
            name: "touch".into(),
            description: file::TouchTool.description().into(),
            parameters: file::TouchTool.definition(),
        },
        ToolDefinition {
            name: "mkdir".into(),
            description: file::MkdirTool.description().into(),
            parameters: file::MkdirTool.definition(),
        },
        ToolDefinition {
            name: "grep".into(),
            description: file::GrepTool.description().into(),
            parameters: file::GrepTool.definition(),
        },
        ToolDefinition {
            name: "glob".into(),
            description: file::GlobTool.description().into(),
            parameters: file::GlobTool.definition(),
        },
        ToolDefinition {
            name: "file_info".into(),
            description: file::FileInfoTool.description().into(),
            parameters: file::FileInfoTool.definition(),
        },
        ToolDefinition {
            name: "find".into(),
            description: file::FindTool.description().into(),
            parameters: file::FindTool.definition(),
        },
        ToolDefinition {
            name: "cat".into(),
            description: file::CatTool.description().into(),
            parameters: file::CatTool.definition(),
        },
        ToolDefinition {
            name: "read_multiple_files".into(),
            description: file::ReadMultipleFilesTool.description().into(),
            parameters: file::ReadMultipleFilesTool.definition(),
        },
        // Plan tools
        ToolDefinition {
            name: "ask_user".into(),
            description: plan::AskUserTool.description().into(),
            parameters: plan::AskUserTool.definition(),
        },
        ToolDefinition {
            name: "exit_plan_mode".into(),
            description: plan::ExitPlanModeTool.description().into(),
            parameters: plan::ExitPlanModeTool.definition(),
        },
        ToolDefinition {
            name: "plan_write".into(),
            description: plan::PlanWriteTool.description().into(),
            parameters: plan::PlanWriteTool.definition(),
        },
        ToolDefinition {
            name: "plan_read".into(),
            description: plan::PlanReadTool.description().into(),
            parameters: plan::PlanReadTool.definition(),
        },
        ToolDefinition {
            name: "plan_edit".into(),
            description: plan::PlanEditTool.description().into(),
            parameters: plan::PlanEditTool.definition(),
        },
        ToolDefinition {
            name: "plan_list".into(),
            description: plan::PlanListTool.description().into(),
            parameters: plan::PlanListTool.definition(),
        },
        // Memory tools
        ToolDefinition {
            name: "memory_set".into(),
            description: memory::MemorySetTool.description().into(),
            parameters: memory::MemorySetTool.definition(),
        },
        ToolDefinition {
            name: "memory_get".into(),
            description: memory::MemoryGetTool.description().into(),
            parameters: memory::MemoryGetTool.definition(),
        },
        ToolDefinition {
            name: "memory_search".into(),
            description: memory::MemorySearchTool.description().into(),
            parameters: memory::MemorySearchTool.definition(),
        },
        ToolDefinition {
            name: "memory_delete".into(),
            description: memory::MemoryDeleteTool.description().into(),
            parameters: memory::MemoryDeleteTool.definition(),
        },
        ToolDefinition {
            name: "memory_list".into(),
            description: memory::MemoryListTool.description().into(),
            parameters: memory::MemoryListTool.definition(),
        },
        // Web tools
        ToolDefinition {
            name: "web_fetch".into(),
            description: web::WebFetchTool.description().into(),
            parameters: web::WebFetchTool.definition(),
        },
        ToolDefinition {
            name: "web_search".into(),
            description: web::WebSearchTool.description().into(),
            parameters: web::WebSearchTool.definition(),
        },
    ]
}
