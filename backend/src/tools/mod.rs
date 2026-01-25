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

use crate::{DbConn, error::{Error, Result}, models::requests::ToolResponse};
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;

/// Tool trait for extensible toolset
///
/// All tools implement this trait to provide a unified execution interface.
/// This allows for easy addition of new tools to the system.
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
    /// * `workspace_id` - ID of workspace to operate on
    /// * `user_id` - ID of authenticated user executing the tool
    /// * `args` - Tool-specific arguments as JSON value
    ///
    /// # Returns
    /// Tool response with success status and result or error
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        user_id: Uuid,
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
}

impl ToolExecutor {
    pub async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        match self {
            ToolExecutor::Ls => ls::LsTool.execute(conn, workspace_id, user_id, args).await,
            ToolExecutor::Read => read::ReadTool.execute(conn, workspace_id, user_id, args).await,
            ToolExecutor::Write => write::WriteTool.execute(conn, workspace_id, user_id, args).await,
            ToolExecutor::Rm => rm::RmTool.execute(conn, workspace_id, user_id, args).await,
            ToolExecutor::Mv => mv::MvTool.execute(conn, workspace_id, user_id, args).await,
            ToolExecutor::Touch => touch::TouchTool.execute(conn, workspace_id, user_id, args).await,
            ToolExecutor::Edit => edit::EditTool.execute(conn, workspace_id, user_id, args).await,
        }
    }
}
