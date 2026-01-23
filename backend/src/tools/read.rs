use crate::{DbConn, error::Result, Error};
use crate::models::requests::{ToolResponse, ReadArgs, ReadResult};
use crate::services::files;
use crate::queries::files as file_queries;
use uuid::Uuid;
use serde_json::Value;
use super::Tool;

/// Read file contents tool
///
/// Reads the latest version of a file within a workspace.
pub struct ReadTool;

impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        _user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let read_args: ReadArgs = serde_json::from_value(args)?;
        
        let file = file_queries::get_file_by_path(conn, workspace_id, &read_args.path)
            .await?
            .ok_or_else(|| crate::error::Error::NotFound(format!("File not found: {}", read_args.path)))?;
        
        let file_with_content = files::get_file_with_content(conn, file.id).await?;
        
        let result = ReadResult {
            path: read_args.path,
            content: file_with_content.latest_version.content_raw,
        };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
