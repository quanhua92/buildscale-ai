use crate::{DbConn, error::{Result, Error}};
use crate::models::requests::{ToolResponse, ReadArgs, ReadResult};
use crate::services::files;
use crate::queries::files as file_queries;
use crate::tools::helpers;
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
        r#"Reads up to 500 lines by default. Use offset to control position (positive=from start, negative=from end). Example: offset=-100 reads last 100 lines. Use limit to control max lines.

SCROLL MODE (for navigating large files):
- Set cursor to enable scroll mode (e.g., cursor=100 starts at line 100)
- offset becomes relative to cursor (e.g., cursor=100, offset=-50 reads lines 50-100)
- Positive offset scrolls down, negative offset scrolls up
- Returns cursor field showing position at end of read for next scroll operation

EXAMPLES:
- Read first 500 lines: {"path": "/file.txt"}
- Read last 100 lines: {"path": "/file.txt", "offset": -100, "limit": 100}
- Scroll mode - start at line 1000, read 100 lines: {"path": "/file.txt", "cursor": 1000, "offset": 0, "limit": 100}
- Scroll up 50 lines from cursor 200: {"path": "/file.txt", "cursor": 200, "offset": -50, "limit": 50}

Returns content, hash for change detection, and metadata (total_lines, truncated flag, cursor position)."#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "offset": {
                    "type": ["integer", "string"],
                    "description": "Starting line position (default: 0). Accepts integer or string (e.g., 100 or '100'). Positive values from start (e.g., 100 = line 100+). Negative values from end (e.g., -100 = last 100 lines). In scroll mode (with cursor), offset is relative to cursor."
                },
                "limit": {
                    "type": ["integer", "string"],
                    "description": "Maximum number of lines to read. Accepts integer or string (e.g., 500 or '500'). Default: 500"
                },
                "cursor": {
                    "type": ["integer", "string", "null"],
                    "description": "Optional cursor position (line number) for scroll mode. Accepts integer or string (e.g., 100 or '100'). When set, offset becomes relative to cursor. Enables navigation of large files without calculating absolute positions."
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
        let cursor = read_args.cursor;

        // Try database lookup first
        let file = match file_queries::get_file_by_path(conn, workspace_id, &path).await? {
            Some(f) => {
                tracing::debug!(workspace_id = %workspace_id, path = %path, "File found in database");
                f
            },
            None => {
                tracing::debug!(workspace_id = %workspace_id, path = %path, "File not found in database, checking filesystem");
                // Fallback: Check if file exists on disk
                match helpers::file_exists_on_disk(storage, workspace_id, &path).await {
                    Ok(true) => {
                        tracing::debug!(workspace_id = %workspace_id, path = %path, "File exists on disk, reading from filesystem");
                        // File exists on disk but not in database - read from disk
                        let (content, hash) = helpers::read_file_from_disk(
                            storage,
                            workspace_id,
                            &path,
                        ).await?;

                        tracing::debug!(workspace_id = %workspace_id, path = %path, content_length = content.len(), "Successfully read file from disk");

                        // For disk-only files, return immediately with basic metadata
                        // We can't support scroll mode or line counting for unsynced files
                        let result = ReadResult {
                            path: path.clone(),
                            content: serde_json::json!(content),
                            hash,
                            synced: false,  // Filesystem-only
                            total_lines: Some(content.lines().count()),
                            truncated: Some(false),
                            offset: Some(0),
                            limit: Some(limit),
                            cursor: None,  // Scroll mode not supported for unsynced files
                        };

                        return Ok(ToolResponse {
                            success: true,
                            result: serde_json::to_value(result)?,
                            error: None,
                        });
                    }
                    Ok(false) => {
                        tracing::debug!(workspace_id = %workspace_id, path = %path, "File not found on disk either");
                        // File not found in database or on disk
                        return Err(Error::NotFound(format!("File not found: {}", path)));
                    }
                    Err(e) => {
                        tracing::error!(workspace_id = %workspace_id, path = %path, error = %e.to_string(), "Error checking if file exists on disk");
                        // Error checking disk - return the error
                        return Err(e);
                    }
                }
            }
        };

        if matches!(file.file_type, crate::models::files::FileType::Folder) {
            return Err(crate::error::Error::Validation(crate::error::ValidationErrors::Single {
                field: "path".to_string(),
                message: "Cannot read content of a folder".to_string(),
            }));
        }

        let file_with_content = files::get_file_with_content(conn, storage, file.id).await?;

        // Calculate offset based on mode (cursor vs absolute)
        let (calculated_offset, cursor_mode) = if let Some(cursor_pos) = cursor {
            // Scroll mode: offset is relative to cursor
            let relative_offset = if offset < 0 {
                // Scroll up: negative offset from cursor
                let up_lines = offset.abs() as usize;
                if up_lines >= cursor_pos {
                    0 // Can't scroll past beginning
                } else {
                    cursor_pos - up_lines
                }
            } else {
                // Scroll down: positive offset from cursor
                cursor_pos + offset as usize
            };
            (relative_offset, true)
        } else {
            // Absolute offset mode
            let abs_offset = if offset < 0 {
                // Negative offset: read from end
                0 // Will be handled in slice_content_from_end
            } else {
                offset as usize
            };
            (abs_offset, false)
        };

        // Apply offset/limit for string content
        let (content, total_lines, truncated) = match &file_with_content.content {
            serde_json::Value::String(s) => {
                let (sliced, total, was_truncated) = if cursor_mode {
                    // Scroll mode: always use positive offset from beginning
                    slice_content_by_lines(s, calculated_offset, limit)
                } else {
                    // Absolute mode: handle negative offset for reading from end
                    if offset < 0 {
                        let lines_from_end = offset.abs() as usize;
                        slice_content_from_end(s, lines_from_end, limit)
                    } else {
                        slice_content_by_lines(s, calculated_offset, limit)
                    }
                };
                (serde_json::Value::String(sliced), Some(total), Some(was_truncated))
            }
            other => (other.clone(), None, None), // Non-string content returned as-is
        };

        // Calculate actual offset for result (convert negative to actual position)
        let actual_offset = if offset < 0 && cursor.is_none() {
            let total = total_lines.unwrap_or(0);
            let abs_offset = offset.abs() as usize;
            if abs_offset >= total { 0 } else { total - abs_offset }
        } else {
            calculated_offset
        };

        // Calculate new cursor position (end of current read)
        let total_lines_read = content.as_str()
            .map(|s| s.lines().count())
            .unwrap_or(0);
        let new_cursor = actual_offset + total_lines_read;

        let result = ReadResult {
            path,
            content,
            hash: file_with_content.latest_version.hash,
            synced: true,  // Database entry
            total_lines,
            truncated,
            offset: Some(actual_offset),
            limit: Some(limit),
            cursor: Some(new_cursor),
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
