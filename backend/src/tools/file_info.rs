use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, FileInfoArgs, FileInfoResult}, queries::files as file_queries};
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::tools::helpers;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// File info tool for metadata queries
///
/// Gets file metadata without reading full content (token efficient).
pub struct FileInfoTool;

#[async_trait]
impl Tool for FileInfoTool {
    fn name(&self) -> &'static str {
        "file_info"
    }

    fn description(&self) -> &'static str {
        r#"Gets file metadata without reading full content. Returns size, line_count, timestamps, hash.

EXAMPLE: {"path":"/file.txt"}"#
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
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let file_info_args: FileInfoArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&file_info_args.path);

        // Try database lookup first
        let file = match file_queries::get_file_by_path(conn, workspace_id, &path).await? {
            Some(f) => f,
            None => {
                // Fallback: Check if file exists on disk
                match helpers::file_exists_on_disk(storage, workspace_id, &path).await {
                    Ok(true) => {
                        // File exists on disk but not in database - get metadata from disk
                        let (size, updated_at) = helpers::get_file_metadata_from_disk(
                            storage,
                            workspace_id,
                            &path,
                        ).await?;

                        // Read file to get line count for text files
                        let (content, _hash) = helpers::read_file_from_disk(
                            storage,
                            workspace_id,
                            &path,
                        ).await?;

                        let line_count = Some(content.lines().count());

                        let result = FileInfoResult {
                            path,
                            file_type: crate::models::files::FileType::Document,  // Default for disk files
                            size: Some(size),
                            line_count,
                            synced: false,  // Filesystem-only
                            created_at: updated_at,  // Use updated_at as fallback
                            updated_at,
                            hash: _hash,
                        };

                        return Ok(ToolResponse {
                            success: true,
                            result: serde_json::to_value(result)?,
                            error: None,
                        });
                    }
                    Ok(false) => {
                        // File not found in database or on disk
                        return Err(Error::NotFound(format!("File not found: {}", path)));
                    }
                    Err(e) => {
                        // Error checking disk - return the error
                        return Err(e);
                    }
                }
            }
        };

        // Get file size from filesystem using workspace path
        let workspace_path = storage.get_workspace_path(workspace_id);
        let relative_path = path.strip_prefix('/').unwrap_or(&path);
        let file_path = workspace_path.join(relative_path);

        let size = if !matches!(file.file_type, crate::models::files::FileType::Folder) {
            tokio::fs::metadata(&file_path).await
                .map(|metadata| Some(metadata.len() as usize))
                .unwrap_or(None)
        } else {
            None // Folders don't have a size
        };

        // Get line count for text files only
        let line_count = if matches!(file.file_type, crate::models::files::FileType::Document) {
            // Only attempt to get content for document files, not folders
            match files::get_file_with_content(conn, storage, file.id).await {
                Ok(file_with_content) => {
                    if let Some(Value::String(content)) = file_with_content.content.get("text") {
                        Some(content.lines().count())
                    } else if let Some(content) = file_with_content.content.as_str() {
                        Some(content.lines().count())
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        } else {
            // Folders and other file types don't have line counts
            None
        };

        // Get content hash
        let latest_version = file_queries::get_latest_version(conn, file.id).await?;
        let hash = latest_version.hash;

        let result = FileInfoResult {
            path,
            file_type: file.file_type,
            size,
            line_count,
            synced: true,  // Database entry
            created_at: file.created_at,
            updated_at: file.updated_at,
            hash,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
