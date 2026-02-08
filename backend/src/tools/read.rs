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

/// Slices content string starting from the end
/// Returns (sliced_content, total_lines, was_truncated)
fn slice_content_from_end(
    content: &str,
    lines_from_end: usize,
    limit: usize,
) -> (String, usize, bool) {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    // Calculate start position: last 100 of 1000 = start at 900
    let start = if lines_from_end >= total_lines {
        0
    } else {
        total_lines - lines_from_end
    };

    // Apply limit
    let end = (start + limit).min(total_lines);
    let was_truncated = end < total_lines;

    let sliced_lines = lines.get(start..end)
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
        "Reads up to 500 lines by default. Use offset to control position (positive=from start, negative=from end). Example: offset=-100 reads last 100 lines. Use limit to control max lines. Returns content, hash for change detection, and metadata (total_lines, truncated flag). Cannot read folders. Always read before editing to get the latest hash."
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "offset": {
                    "type": "integer",
                    "description": "Starting line position (default: 0). Positive values read from beginning (e.g., 100 = line 100+). Negative values read from end (e.g., -100 = last 100 lines)."
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
                let (sliced, total, was_truncated) = if offset < 0 {
                    // Negative offset: read from end
                    let lines_from_end = offset.abs() as usize;
                    slice_content_from_end(s, lines_from_end, limit)
                } else {
                    // Positive/zero offset: read from beginning
                    slice_content_by_lines(s, offset as usize, limit)
                };
                (serde_json::Value::String(sliced), Some(total), Some(was_truncated))
            }
            other => (other.clone(), None, None), // Non-string content returned as-is
        };

        // Calculate actual offset for result (convert negative to actual position)
        let actual_offset = if offset < 0 {
            let total = total_lines.unwrap_or(0);
            let abs_offset = offset.abs() as usize;
            if abs_offset >= total { 0 } else { total - abs_offset }
        } else {
            offset as usize
        };

        let result = ReadResult {
            path,
            content,
            hash: file_with_content.latest_version.hash,
            total_lines,
            truncated,
            offset: Some(actual_offset),
            limit: Some(limit),
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
