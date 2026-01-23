//! Tool system for BuildScale
//!
//! This module provides an extensible toolset that operates on files in workspaces.
//! Tools follow the "Everything is a File" philosophy, providing filesystem-like
//! operations (ls, read, write, rm) backed by the database.

pub mod ls;
pub mod read;
pub mod write;
pub mod rm;

use crate::{DbConn, error::{Error, Result}, models::requests::ToolResponse};
use uuid::Uuid;
use serde_json::Value;

/// Tool trait for extensible toolset
///
/// All tools implement this trait to provide a unified execution interface.
/// This allows for easy addition of new tools to the system.
pub trait Tool: Send + Sync {
    /// Returns the name of this tool
    fn name(&self) -> &'static str;
    
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

/// Tool context passed to execute()
///
/// Contains workspace and user context for tool execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

/// Get tool by name from registry
///
/// Returns a closure that executes the tool when called with the appropriate arguments
///
/// # Arguments
/// * `tool_name` - Name of the tool to retrieve (e.g., "ls", "read", "write", "rm")
///
/// # Returns
/// * `Ok(ToolExecutor)` - The tool executor closure
/// * `Err(Error)` - If tool name is not found
///
/// # Example
/// ```no_run
/// use buildscale::tools::get_tool_executor;
///
/// let executor = get_tool_executor("read")?;
/// executor.execute(conn, workspace_id, user_id, args).await?;
/// ```
pub fn get_tool_executor(tool_name: &str) -> Result<ToolExecutor> {
    match tool_name {
        "ls" => Ok(ToolExecutor::Ls),
        "read" => Ok(ToolExecutor::Read),
        "write" => Ok(ToolExecutor::Write),
        "rm" => Ok(ToolExecutor::Rm),
        _ => Err(Error::NotFound(format!("Tool '{}' not found", tool_name))),
    }
}

/// Tool executor enum for dispatching tool execution
pub enum ToolExecutor {
    Ls,
    Read,
    Write,
    Rm,
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
        }
    }
}
