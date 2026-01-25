use crate::error::Error;
use crate::models::requests::{LsArgs, MvArgs, ReadArgs, RmArgs, TouchArgs, WriteArgs};
use crate::tools::{self, Tool};
use crate::DbPool;
use rig::completion::ToolDefinition;
use rig::tool::Tool as RigTool;
use std::future::Future;
use uuid::Uuid;

fn enforce_strict_schema(mut schema: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = schema.as_object_mut() {
        obj.insert("additionalProperties".to_string(), serde_json::json!(false));

        // Ensure all properties are in 'required' list for OpenAI strict mode
        if let Some(properties) = obj.get("properties").and_then(|p| p.as_object()) {
            let all_keys: Vec<String> = properties.keys().cloned().collect();
            obj.insert("required".to_string(), serde_json::json!(all_keys));
        }
    }
    schema
}

/// A Rig-compatible wrapper for BuildScale tools.
pub struct RigLsTool {
    pub pool: DbPool,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

impl RigTool for RigLsTool {
    type Error = Error;
    type Args = LsArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "ls";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            ToolDefinition {
                name,
                description: "Lists files and folders in a directory within the workspace."
                    .to_string(),
                parameters: enforce_strict_schema(
                    serde_json::to_value(schemars::schema_for!(LsArgs)).unwrap_or_default(),
                ),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
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

impl RigTool for RigReadTool {
    type Error = Error;
    type Args = ReadArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "read";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            ToolDefinition {
                name,
                description: "Reads the literal content of a file at the specified path."
                    .to_string(),
                parameters: enforce_strict_schema(
                    serde_json::to_value(schemars::schema_for!(ReadArgs)).unwrap_or_default(),
                ),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
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

impl RigTool for RigWriteTool {
    type Error = Error;
    type Args = WriteArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "write";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            ToolDefinition {
                name,
                description: "Creates or updates a file at the specified path with the provided content."
                    .to_string(),
                parameters: enforce_strict_schema(
                    serde_json::to_value(schemars::schema_for!(WriteArgs)).unwrap_or_default(),
                ),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        let pool = self.pool.clone();
        let workspace_id = self.workspace_id;
        let user_id = self.user_id;

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

impl RigTool for RigRmTool {
    type Error = Error;
    type Args = RmArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "rm";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            ToolDefinition {
                name,
                description: "Deletes a file or empty folder at the specified path.".to_string(),
                parameters: enforce_strict_schema(
                    serde_json::to_value(schemars::schema_for!(RmArgs)).unwrap_or_default(),
                ),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
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

pub struct RigMvTool {
    pub pool: DbPool,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

impl RigTool for RigMvTool {
    type Error = Error;
    type Args = MvArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "mv";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            ToolDefinition {
                name,
                description: "Moves or renames a file. To rename, provide the full new path. To move, provide the new parent directory path.".to_string(),
                parameters: enforce_strict_schema(
                    serde_json::to_value(schemars::schema_for!(MvArgs)).unwrap_or_default(),
                ),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        let pool = self.pool.clone();
        let workspace_id = self.workspace_id;
        let user_id = self.user_id;
        async move {
            let mut conn = pool.acquire().await.map_err(Error::Sqlx)?;
            let tool = tools::mv::MvTool;
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

pub struct RigTouchTool {
    pub pool: DbPool,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

impl RigTool for RigTouchTool {
    type Error = Error;
    type Args = TouchArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "touch";

    fn definition(&self, _prompt: String) -> impl Future<Output = ToolDefinition> + Send + Sync {
        let name = Self::NAME.to_string();
        async move {
            ToolDefinition {
                name,
                description: "Updates the access and modification times of a file, or creates an empty file if it doesn't exist.".to_string(),
                parameters: enforce_strict_schema(
                    serde_json::to_value(schemars::schema_for!(TouchArgs)).unwrap_or_default(),
                ),
            }
        }
    }

    fn call(
        &self,
        args: Self::Args,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        let pool = self.pool.clone();
        let workspace_id = self.workspace_id;
        let user_id = self.user_id;
        async move {
            let mut conn = pool.acquire().await.map_err(Error::Sqlx)?;
            let tool = tools::touch::TouchTool;
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
