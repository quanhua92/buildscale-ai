use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, FileInfoArgs, FileInfoResult}, queries::files as file_queries};
use crate::services::files;
use crate::services::storage::FileStorageService;
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
        r#"Gets file metadata efficiently. For text files, reads content to calculate line_count. Returns path, file_type, size, line_count (for text files), created_at, updated_at, and content hash.

USE CASES:
- Check file size before reading large files
- Verify file existence without fetching full content
- Quick file statistics for decision-making
- Get file timestamps for sorting/filtering

COMPARISON:
- file_info: Returns metadata only (fast, token-efficient)
- read: Returns full content (use when you need the file data)

EXAMPLES:
- Check size: {"path": "/large-file.log"}
- Verify exists: {"path": "/config.json"}
- Get timestamps: {"path": "/README.md"}"#
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

        let file = file_queries::get_file_by_path(conn, workspace_id, &path).await?
            .ok_or_else(|| Error::NotFound(format!("File not found: {}", path)))?;

        // Get file size from storage if available
        let size = None; // TODO: Implement size tracking in storage layer

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
