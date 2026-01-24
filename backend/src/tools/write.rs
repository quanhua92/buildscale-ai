use crate::error::{Error, Result, ValidationErrors};
use crate::models::files::FileType;
use crate::models::requests::{
    CreateFileRequest, CreateVersionRequest, ToolResponse, WriteArgs, WriteResult,
};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;
use super::Tool;

/// Write file contents tool
///
/// Creates a new file or updates an existing file with new content.
pub struct WriteTool;

#[async_trait]
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
        let path = super::normalize_path(&write_args.path);
        
        let existing_file = file_queries::get_file_by_path(conn, workspace_id, &path).await?;
        
        let result = if let Some(file) = existing_file {
            // Validation: Ensure content matches existing file type
            Self::validate_content_for_type(file.file_type, &write_args.content, write_args.file_type.as_deref())?;

            let version = files::create_version(conn, file.id, CreateVersionRequest {
                author_id: Some(user_id),
                branch: Some("main".to_string()),
                content: write_args.content,
                app_data: None,
            }).await?;
            
            WriteResult {
                path,
                file_id: file.id,
                version_id: version.id,
            }
        } else {
            let filename = path.rsplit('/').next().unwrap_or("untitled");
            
            let file_type = if let Some(ft_str) = write_args.file_type.as_deref() {
                FileType::from_str(ft_str).map_err(|_| {
                    Error::Validation(ValidationErrors::Single {
                        field: "file_type".to_string(),
                        message: format!("Invalid file type: {}", ft_str),
                    })
                })?
            } else {
                FileType::Document
            };

            // Validation: Ensure content matches new file type
            Self::validate_content_for_type(file_type, &write_args.content, write_args.file_type.as_deref())?;

            let file_result = files::create_file_with_content(conn, CreateFileRequest {
                workspace_id,
                parent_id: None,
                author_id: user_id,
                name: filename.to_string(),
                slug: None,
                path: Some(path.clone()),
                file_type,
                content: write_args.content,
                app_data: None,
            }).await?;
            
            WriteResult {
                path,
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

impl WriteTool {
    /// Validates that the content structure matches the requirements for the given file type.
    fn validate_content_for_type(
        actual_type: FileType,
        content: &Value,
        requested_type_str: Option<&str>,
    ) -> Result<()> {
        // 1. Prevent writing text content to a folder path unless explicitly creating a folder
        if matches!(actual_type, FileType::Folder) && requested_type_str != Some("folder") {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: "Cannot write text content to a folder path".to_string(),
            }));
        }

        // 2. Validate Document schema (must have a "text" field with a string value)
        if matches!(actual_type, FileType::Document) {
            if !content.get("text").map_or(false, |v| v.is_string()) {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "content".to_string(),
                    message: "Document content must contain a 'text' field with a string value"
                        .to_string(),
                }));
            }
        }

        Ok(())
    }
}
