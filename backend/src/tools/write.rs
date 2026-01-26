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

    fn description(&self) -> &'static str {
        "Creates a new file or completely replaces existing file content. For Document files, raw string content is auto-wrapped into {\"text\": \"...\"} format. CRITICAL: This is NOT for partial edits - use 'edit' tool to modify specific sections. Use 'write' only for new files or complete file replacement."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(WriteArgs)).unwrap_or(Value::Null)
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
        
        // Virtual File Protection: Prevent direct writes to system-managed files (e.g. Chats)
        if let Some(ref file) = existing_file {
            if file.is_virtual {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "path".to_string(),
                    message: "Cannot write to a virtual file directly. Use specialized system tools (e.g., chat API) to modify this resource.".to_string(),
                }));
            }
        }

        let result = if let Some(file) = existing_file {
            // Prepare content: handle auto-wrapping for documents
            let final_content = Self::prepare_content_for_type(file.file_type, write_args.content, write_args.file_type.as_deref())?;

            let version = files::create_version(conn, file.id, CreateVersionRequest {
                author_id: Some(user_id),
                branch: Some("main".to_string()),
                content: final_content,
                app_data: None,
            }).await?;
            
            WriteResult {
                path,
                file_id: file.id,
                version_id: version.id,
                hash: version.hash,
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

            // Prepare content: handle auto-wrapping for documents
            let final_content = Self::prepare_content_for_type(file_type, write_args.content, write_args.file_type.as_deref())?;

            let file_result = files::create_file_with_content(conn, CreateFileRequest {
                workspace_id,
                parent_id: None,
                author_id: user_id,
                name: filename.to_string(),
                slug: None,
                path: Some(path.clone()),
                is_virtual: None,
                is_remote: None,
                permission: None,
                file_type,
                content: final_content,
                app_data: None,
            }).await?;
            
            WriteResult {
                path,
                file_id: file_result.file.id,
                version_id: file_result.latest_version.id,
                hash: file_result.latest_version.hash,
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
    /// Validates and normalizes content based on the file type.
    /// Handles auto-wrapping raw strings into the expected JSON structure for Documents.
    fn prepare_content_for_type(
        actual_type: FileType,
        content: Value,
        requested_type_str: Option<&str>,
    ) -> Result<Value> {
        // 1. Prevent writing text content to a folder path unless explicitly creating a folder
        if matches!(actual_type, FileType::Folder) && requested_type_str != Some("folder") {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: "Cannot write text content to a folder path".to_string(),
            }));
        }

        // 2. Handle Document normalization and validation
        if matches!(actual_type, FileType::Document) {
            // Auto-wrap raw strings: "hello" -> {"text": "hello"}
            if content.is_string() {
                return Ok(serde_json::json!({ "text": content.as_str().unwrap() }));
            }

            // Check if 'text' field exists
            if !content.get("text").is_some_and(|v| v.is_string()) {
                // If 'text' field is missing entirely
                if content.get("text").is_none() {
                    return Err(Error::Validation(ValidationErrors::Single {
                        field: "content".to_string(),
                        message: "Document content must contain a 'text' field".to_string(),
                    }));
                }
                // If 'text' field exists but is not a string
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "content".to_string(),
                    message: "Document content must contain a 'text' field with a string value".to_string(),
                }));
            }
        }

        Ok(content)
    }
}
