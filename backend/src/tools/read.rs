use crate::{DbConn, error::Result};
use crate::models::requests::{ToolResponse, ReadArgs, ReadResult};
use crate::services::files;
use crate::queries::files as file_queries;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Default maximum number of lines to read from a file.
/// This balances token efficiency with coverage - most files are under 500 lines.
/// For large files, use the offset and limit parameters to read specific sections.
const DEFAULT_READ_LIMIT: usize = 500;

/// Slices content string by lines with offset and limit
/// Returns (sliced_content, total_lines, was_truncated)
fn slice_content_by_lines(
    content: &str,
    offset: usize,
    limit: usize,
) -> (String, usize, bool) {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let was_truncated = offset + limit < total_lines;

    let end = (offset + limit).min(total_lines);
    let sliced_lines = lines.get(offset..end)
        .unwrap_or(&[])
        .join("\n");

    (sliced_lines, total_lines, was_truncated)
}

/// Read file contents tool
///
/// Reads the latest version of a file within a workspace.
pub struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Reads up to 500 lines of a file by default. Use offset (0-indexed) and limit parameters to read specific portions. Returns content, hash for change detection, and metadata (total_lines, truncated flag). For text files, content is returned with line numbering preserved. Cannot read folders. Always read before editing to get the latest hash."
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "offset": {
                    "type": "integer",
                    "description": "Starting line offset (0-indexed). Default: 0"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read. Default: 500"
                }
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
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let read_args: ReadArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&read_args.path);

        // Apply defaults
        let offset = read_args.offset.unwrap_or(0);
        let limit = read_args.limit.unwrap_or(DEFAULT_READ_LIMIT);

        let file = file_queries::get_file_by_path(conn, workspace_id, &path)
            .await?
            .ok_or_else(|| crate::error::Error::NotFound(format!("File not found: {}", path)))?;

        if matches!(file.file_type, crate::models::files::FileType::Folder) {
            return Err(crate::error::Error::Validation(crate::error::ValidationErrors::Single {
                field: "path".to_string(),
                message: "Cannot read content of a folder".to_string(),
            }));
        }

        let file_with_content = files::get_file_with_content(conn, storage, file.id).await?;

        // Apply offset/limit for string content
        let (content, total_lines, truncated) = match &file_with_content.content {
            serde_json::Value::String(s) => {
                let (sliced, total, was_truncated) =
                    slice_content_by_lines(s, offset, limit);
                (serde_json::Value::String(sliced), Some(total), Some(was_truncated))
            }
            other => (other.clone(), None, None), // Non-string content returned as-is
        };

        let result = ReadResult {
            path,
            content,
            hash: file_with_content.latest_version.hash,
            total_lines,
            truncated,
            offset: Some(offset),
            limit: Some(limit),
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
