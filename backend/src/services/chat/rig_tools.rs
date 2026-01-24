use crate::error::Error;
use crate::tools::{self, Tool};
use crate::DbPool;
use rig::completion::ToolDefinition;
use rig::tool::Tool as RigTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::future::Future;

/// A Rig-compatible wrapper for BuildScale tools.
pub struct RigLsTool {
    pub pool: DbPool,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct LsArgs {
    /// The path to list. Defaults to root "/" if not provided.
    pub path: Option<String>,
    /// Whether to list files recursively.
    pub recursive: Option<bool>,
}

impl RigTool for RigLsTool {
    type Error = Error;
    type Args = LsArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "ls";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            let params = serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to list. Defaults to root '/' if not provided."
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to list files recursively."
                    }
                },
                "required": ["path", "recursive"],
                "additionalProperties": false
            });

            ToolDefinition {
                name,
                description: "Lists files and folders in a directory within the workspace.".to_string(),
                parameters: params,
            }
        }
    }

    fn call(&self, args: Self::Args) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        let pool = self.pool.clone();
        let workspace_id = self.workspace_id;
        let user_id = self.user_id;
        async move {
            let mut conn = pool.acquire().await.map_err(Error::Sqlx)?;
            let tool = tools::ls::LsTool;
            let response = tool
                .execute(
                    &mut conn,
                    workspace_id,
                    user_id,
                    serde_json::to_value(args)?,
                )
                .await?;
            if response.success {
                Ok(response.result)
            } else {
                Err(Error::Internal(
                    response.error.unwrap_or_else(|| "Unknown tool error".to_string()),
                ))
            }
        }
    }
}

pub struct RigReadTool {
    pub pool: DbPool,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadArgs {
    /// The absolute path of the file to read.
    pub path: String,
}

impl RigTool for RigReadTool {
    type Error = Error;
    type Args = ReadArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "read";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            let params = serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The absolute path of the file to read."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            });

            ToolDefinition {
                name,
                description: "Reads the literal content of a file at the specified path.".to_string(),
                parameters: params,
            }
        }
    }

    fn call(&self, args: Self::Args) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        let pool = self.pool.clone();
        let workspace_id = self.workspace_id;
        let user_id = self.user_id;
        async move {
            let mut conn = pool.acquire().await.map_err(Error::Sqlx)?;
            let tool = tools::read::ReadTool;
            let response = tool
                .execute(
                    &mut conn,
                    workspace_id,
                    user_id,
                    serde_json::to_value(args)?,
                )
                .await?;
            if response.success {
                Ok(response.result)
            } else {
                Err(Error::Internal(
                    response.error.unwrap_or_else(|| "Unknown tool error".to_string()),
                ))
            }
        }
    }
}

pub struct RigWriteTool {
    pub pool: DbPool,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WriteArgs {
    /// The absolute path where the file should be created or updated.
    pub path: String,
    /// The content to write. For documents, this should be a JSON object like {"text": "..."}.
    pub content: serde_json::value::Value,
    /// Optional file type (e.g., "document", "canvas"). Defaults to "document".
    pub file_type: Option<String>,
}

impl RigTool for RigWriteTool {
    type Error = Error;
    type Args = WriteArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "write";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            let params = serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The absolute path where the file should be created or updated."
                    },
                    "content": {
                        "type": "string",
                        "description": "The text content to write to the file."
                    },
                    "file_type": {
                        "type": "string",
                        "description": "Optional file type (e.g., 'document', 'canvas').",
                        "enum": ["document", "canvas", "chat", "whiteboard", "agent", "skill"]
                    }
                },
                "required": ["path", "content", "file_type"],
                "additionalProperties": false
            });

            ToolDefinition {
                name,
                description: "Creates or updates a file at the specified path with the provided content."
                    .to_string(),
                parameters: params,
            }
        }
    }

    fn call(&self, mut args: Self::Args) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        let pool = self.pool.clone();
        let workspace_id = self.workspace_id;
        let user_id = self.user_id;
        
        // Auto-wrap string content into the expected JSON structure for documents
        if args.content.is_string() {
            let file_type = args.file_type.as_deref().unwrap_or("document");
            if file_type == "document" {
                args.content = serde_json::json!({ "text": args.content.as_str().unwrap() });
            }
        }

        async move {
            let mut conn = pool.acquire().await.map_err(Error::Sqlx)?;
            let tool = tools::write::WriteTool;
            let response = tool
                .execute(
                    &mut conn,
                    workspace_id,
                    user_id,
                    serde_json::to_value(args)?,
                )
                .await?;
            if response.success {
                Ok(response.result)
            } else {
                Err(Error::Internal(
                    response.error.unwrap_or_else(|| "Unknown tool error".to_string()),
                ))
            }
        }
    }
}

pub struct RigRmTool {
    pub pool: DbPool,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RmArgs {
    /// The absolute path of the file or folder to delete.
    pub path: String,
}

impl RigTool for RigRmTool {
    type Error = Error;
    type Args = RmArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "rm";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            let params = serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The absolute path of the file or folder to delete."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            });

            ToolDefinition {
                name,
                description: "Deletes a file or empty folder at the specified path.".to_string(),
                parameters: params,
            }
        }
    }

    fn call(&self, args: Self::Args) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        let pool = self.pool.clone();
        let workspace_id = self.workspace_id;
        let user_id = self.user_id;
        async move {
            let mut conn = pool.acquire().await.map_err(Error::Sqlx)?;
            let tool = tools::rm::RmTool;
            let response = tool
                .execute(
                    &mut conn,
                    workspace_id,
                    user_id,
                    serde_json::to_value(args)?,
                )
                .await?;
            if response.success {
                Ok(response.result)
            } else {
                Err(Error::Internal(
                    response.error.unwrap_or_else(|| "Unknown tool error".to_string()),
                ))
            }
        }
    }
}
