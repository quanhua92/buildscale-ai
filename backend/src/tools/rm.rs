use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, RmArgs, RmResult}, services::files, queries::files as file_queries};
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Delete file tool
///
/// Soft deletes a file or empty folder within a workspace.
pub struct RmTool;

#[async_trait]
impl Tool for RmTool {
    fn name(&self) -> &'static str {
        "rm"
    }

    fn description(&self) -> &'static str {
        "Soft deletes a file or folder at the specified path. The item is marked as deleted but recoverable. Use with caution - this operation cannot be undone through the tool interface. Non-empty folders will fail - delete children first."
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
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let rm_args: RmArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&rm_args.path);

        // Plan Mode Guard: Only allow /plans/ directory operations
        if config.plan_mode && !path.starts_with("/plans/") {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "path".to_string(),
                message: super::PLAN_MODE_ERROR.to_string(),
            }));
        }

        let file = file_queries::get_file_by_path(conn, workspace_id, &path)
            .await?
            .ok_or_else(|| Error::NotFound(format!("File not found: {}", path)))?;

        files::soft_delete_file(conn, storage, file.id).await?;
        
        let result = RmResult {
            path,
            file_id: file.id,
        };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
