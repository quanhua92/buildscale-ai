//! Tool system for BuildScale
//!
//! This module provides an extensible toolset that operates on files in workspaces.
//! Tools follow the "Everything is a File" philosophy, providing filesystem-like
//! operations (ls, read, write, rm, mv, touch) backed by the database.

pub mod ls;
pub mod read;
pub mod write;
pub mod rm;
pub mod mv;
pub mod touch;
pub mod edit;
pub mod grep;
pub mod mkdir;
pub mod ask_user;
pub mod exit_plan_mode;
pub mod glob;
pub mod file_info;
pub mod read_multiple_files;
pub mod find;
pub mod cat;

pub mod helpers;

use crate::{DbConn, error::{Error, Result}, models::requests::ToolResponse, models::chat::ToolDefinition, services::storage::FileStorageService};
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;

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

    // Future extensibility:
    // pub skills: Vec<String>,
    // pub agent_id: Uuid,
    // pub session_id: Uuid,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            plan_mode: false, // Default to Build Mode for normal operation
            active_plan_path: None,
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
        "ls" => Ok(ToolExecutor::Ls),
        "read" => Ok(ToolExecutor::Read),
        "write" => Ok(ToolExecutor::Write),
        "rm" => Ok(ToolExecutor::Rm),
        "mv" => Ok(ToolExecutor::Mv),
        "touch" => Ok(ToolExecutor::Touch),
        "edit" => Ok(ToolExecutor::Edit),
        "grep" => Ok(ToolExecutor::Grep),
        "mkdir" => Ok(ToolExecutor::Mkdir),
        "ask_user" => Ok(ToolExecutor::AskUser),
        "exit_plan_mode" => Ok(ToolExecutor::ExitPlanMode),
        "glob" => Ok(ToolExecutor::Glob),
        "file_info" => Ok(ToolExecutor::FileInfo),
        "read_multiple_files" => Ok(ToolExecutor::ReadMultipleFiles),
        "find" => Ok(ToolExecutor::Find),
        "cat" => Ok(ToolExecutor::Cat),
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
    Ls,
    Read,
    Write,
    Rm,
    Mv,
    Touch,
    Edit,
    Grep,
    Mkdir,
    AskUser,
    ExitPlanMode,
    Glob,
    FileInfo,
    ReadMultipleFiles,
    Find,
    Cat,
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
            ToolExecutor::Ls => "ls",
            ToolExecutor::Read => "read",
            ToolExecutor::Write => "write",
            ToolExecutor::Rm => "rm",
            ToolExecutor::Mv => "mv",
            ToolExecutor::Touch => "touch",
            ToolExecutor::Edit => "edit",
            ToolExecutor::Grep => "grep",
            ToolExecutor::Mkdir => "mkdir",
            ToolExecutor::AskUser => "ask_user",
            ToolExecutor::ExitPlanMode => "exit_plan_mode",
            ToolExecutor::Glob => "glob",
            ToolExecutor::FileInfo => "file_info",
            ToolExecutor::ReadMultipleFiles => "read_multiple_files",
            ToolExecutor::Find => "find",
            ToolExecutor::Cat => "cat",
        };

        let span = tracing::info_span!("tool_execute", tool = name, workspace_id = %workspace_id, user_id = %user_id);
        let _enter = span.enter();

        tracing::debug!(args = %args, "Tool input");

        let result = match self {
            ToolExecutor::Ls => ls::LsTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Read => read::ReadTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Write => write::WriteTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Rm => rm::RmTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Mv => mv::MvTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Touch => touch::TouchTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Edit => edit::EditTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Grep => grep::GrepTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Mkdir => mkdir::MkdirTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::AskUser => ask_user::AskUserTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::ExitPlanMode => exit_plan_mode::ExitPlanModeTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Glob => glob::GlobTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::FileInfo => file_info::FileInfoTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::ReadMultipleFiles => read_multiple_files::ReadMultipleFilesTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Find => find::FindTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
            ToolExecutor::Cat => cat::CatTool.execute(conn, storage, workspace_id, user_id, config.clone(), args).await,
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
        ToolDefinition {
            name: "ls".into(),
            description: ls::LsTool.description().into(),
            parameters: ls::LsTool.definition(),
        },
        ToolDefinition {
            name: "read".into(),
            description: read::ReadTool.description().into(),
            parameters: read::ReadTool.definition(),
        },
        ToolDefinition {
            name: "write".into(),
            description: write::WriteTool.description().into(),
            parameters: write::WriteTool.definition(),
        },
        ToolDefinition {
            name: "edit".into(),
            description: edit::EditTool.description().into(),
            parameters: edit::EditTool.definition(),
        },
        ToolDefinition {
            name: "rm".into(),
            description: rm::RmTool.description().into(),
            parameters: rm::RmTool.definition(),
        },
        ToolDefinition {
            name: "mv".into(),
            description: mv::MvTool.description().into(),
            parameters: mv::MvTool.definition(),
        },
        ToolDefinition {
            name: "touch".into(),
            description: touch::TouchTool.description().into(),
            parameters: touch::TouchTool.definition(),
        },
        ToolDefinition {
            name: "mkdir".into(),
            description: mkdir::MkdirTool.description().into(),
            parameters: mkdir::MkdirTool.definition(),
        },
        ToolDefinition {
            name: "grep".into(),
            description: grep::GrepTool.description().into(),
            parameters: grep::GrepTool.definition(),
        },
        ToolDefinition {
            name: "glob".into(),
            description: glob::GlobTool.description().into(),
            parameters: glob::GlobTool.definition(),
        },
        ToolDefinition {
            name: "file_info".into(),
            description: file_info::FileInfoTool.description().into(),
            parameters: file_info::FileInfoTool.definition(),
        },
        ToolDefinition {
            name: "find".into(),
            description: find::FindTool.description().into(),
            parameters: find::FindTool.definition(),
        },
        ToolDefinition {
            name: "cat".into(),
            description: cat::CatTool.description().into(),
            parameters: cat::CatTool.definition(),
        },
        ToolDefinition {
            name: "read_multiple_files".into(),
            description: read_multiple_files::ReadMultipleFilesTool.description().into(),
            parameters: read_multiple_files::ReadMultipleFilesTool.definition(),
        },
        ToolDefinition {
            name: "ask_user".into(),
            description: ask_user::AskUserTool.description().into(),
            parameters: ask_user::AskUserTool.definition(),
        },
        ToolDefinition {
            name: "exit_plan_mode".into(),
            description: exit_plan_mode::ExitPlanModeTool.description().into(),
            parameters: exit_plan_mode::ExitPlanModeTool.definition(),
        },
    ]
}
