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

use crate::{DbConn, error::{Error, Result}, models::requests::ToolResponse, services::storage::FileStorageService};
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
#[derive(Debug, Clone, Default)]
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

/// Creates a strict JSON Schema for tool parameters
///
/// OpenAI requires `additionalProperties: false` for all function parameters.
/// This helper takes any schemars-generated schema and adds this field automatically.
///
/// # Example
///
/// ```rust
/// use buildscale::tools::strict_tool_schema;
/// use serde_json::json;
///
/// // Instead of manually constructing JSON Schema:
/// // let schema = json!({
/// //     "type": "object",
/// //     "properties": {...},
/// //     "additionalProperties": false  // Easy to forget!
/// // });
///
/// // Use the helper:
/// let schema = strict_tool_schema::<MyToolArgs>();
/// ```
pub fn strict_tool_schema<T>() -> Value
where
    T: schemars::JsonSchema,
{
    let type_name = std::any::type_name::<T>();
    tracing::debug!(tool_type = type_name, "Generating strict tool schema");

    let mut schema = serde_json::to_value(schemars::schema_for!(T))
        .unwrap_or_else(|_| serde_json::json!({"type": "object"}));

    if let Some(obj) = schema.as_object_mut() {
        obj.insert("additionalProperties".into(), serde_json::json!(false));
        tracing::debug!(
            tool_type = type_name,
            has_additional_properties = obj.contains_key("additionalProperties"),
            "Schema generated with additionalProperties"
        );
    }

    tracing::trace!(tool_type = type_name, schema = %serde_json::to_string(&schema).unwrap_or_default(), "Final schema output");
    schema
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
