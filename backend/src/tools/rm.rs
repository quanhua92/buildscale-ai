use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, RmArgs, RmResult}, services::files, queries::files as file_queries};
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::Tool;

/// Delete file tool
///
/// Soft deletes a file or empty folder within a workspace.
pub struct RmTool;

#[async_trait]
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
        let path = super::normalize_path(&rm_args.path);
        
        let file = file_queries::get_file_by_path(conn, workspace_id, &path)
            .await?
            .ok_or_else(|| Error::NotFound(format!("File not found: {}", path)))?;
        
        files::soft_delete_file(conn, file.id).await?;
        
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
