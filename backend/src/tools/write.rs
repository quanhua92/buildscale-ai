use crate::{DbConn, error::Result, Error};
use crate::models::requests::{ToolResponse, WriteArgs, WriteResult, CreateFileRequest, CreateVersionRequest};
use crate::models::files::FileType;
use crate::services::files;
use crate::queries::files as file_queries;
use uuid::Uuid;
use serde_json::Value;
use super::Tool;

/// Write file contents tool
///
/// Creates a new file or updates an existing file with new content.
pub struct WriteTool;

impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "write"
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let write_args: WriteArgs = serde_json::from_value(args)?;
        
        let existing_file = file_queries::get_file_by_path(conn, workspace_id, &write_args.path).await?;
        
        let result = if let Some(file) = existing_file {
            let version = files::create_version(conn, file.id, CreateVersionRequest {
                author_id: Some(user_id),
                branch: Some("main".to_string()),
                content: write_args.content,
                app_data: None,
            }).await?;
            
            WriteResult {
                path: write_args.path.clone(),
                file_id: file.id,
                version_id: version.id,
            }
        } else {
            let filename = write_args.path.rsplit('/').next().unwrap_or("untitled");
            
            let file_result = files::create_file_with_content(conn, CreateFileRequest {
                workspace_id,
                parent_id: None,
                author_id: user_id,
                name: filename.to_string(),
                slug: None,
                path: Some(write_args.path.clone()),
                file_type: FileType::Document,
                content: write_args.content,
                app_data: None,
            }).await?;
            
            WriteResult {
                path: write_args.path.clone(),
                file_id: file_result.file.id,
                version_id: file_result.latest_version.id,
            }
        };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
