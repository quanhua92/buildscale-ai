use crate::{DbConn, error::{Error, Result}};
use crate::models::requests::{ToolResponse, MkdirArgs, MkdirResult};
use crate::services::files as file_services;
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Mkdir tool for creating directories
///
/// Recursively creates folders to ensure a path exists.
pub struct MkdirTool;

#[async_trait]
impl Tool for MkdirTool {
    fn name(&self) -> &'static str {
        "mkdir"
    }

    fn description(&self) -> &'static str {
        "Recursively creates directories to ensure the full path exists. Creates all parent directories automatically if they don't exist. For example, 'mkdir /a/b/c' will create /a, /a/b, and /a/b/c as needed. Succeeds silently if path already exists."
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            },
            "required": ["path"],
            "additionalProperties": false
        })
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let mkdir_args: MkdirArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&mkdir_args.path);

        // Plan Mode Guard: Only allow /plans/ directory operations
        if config.plan_mode && !path.starts_with("/plans/") {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "path".to_string(),
                message: super::PLAN_MODE_ERROR.to_string(),
            }));
        }

        // Create the folder in the database (ensures parent folders exist)
        let folder_id = file_services::ensure_path_exists(
            conn,
            workspace_id,
            &path,
            user_id
        ).await?;

        // Create the actual directory on disk
        storage.create_folder(workspace_id, &path).await?;

        let result = MkdirResult {
            path,
            file_id: folder_id,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
