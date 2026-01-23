use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, RmArgs, RmResult}, services::files, queries::files as file_queries};
use uuid::Uuid;
use serde_json::Value;
use super::Tool;

/// Delete file tool
///
/// Soft deletes a file or empty folder within a workspace.
pub struct RmTool;

impl Tool for RmTool {
    fn name(&self) -> &'static str {
        "rm"
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        _user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let rm_args: RmArgs = serde_json::from_value(args)?;
        
        let file = file_queries::get_file_by_path(conn, workspace_id, &rm_args.path)
            .await?
            .ok_or_else(|| Error::NotFound(format!("File not found: {}", rm_args.path)))?;
        
        files::soft_delete_file(conn, file.id).await?;
        
        let result = RmResult {
            path: rm_args.path,
            file_id: file.id,
        };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
