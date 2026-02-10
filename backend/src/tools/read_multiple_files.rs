use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, ReadMultipleFilesArgs, ReadMultipleFilesResult, ReadFileResult}, queries::files as file_queries, services::files};
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Read multiple files tool for parallel bulk reads
///
/// Reads multiple files, returning results for each file.
/// Note: Due to database connection constraints, files are read sequentially.
pub struct ReadMultipleFilesTool;

#[async_trait]
impl Tool for ReadMultipleFilesTool {
    fn name(&self) -> &'static str {
        "read_multiple_files"
    }

    fn description(&self) -> &'static str {
        r#"Reads multiple files in a single tool call. Reduces round-trips when scanning multiple files. Returns per-file success/error status.

USE CASES:
- Batch file analysis across multiple files
- Cross-referencing content in different files
- Collecting data from logs, configs, or documentation
- Comparing files side-by-side

FEATURES:
- Single tool call for multiple files (reduced network round-trips)
- Partial success handling (some files can fail while others succeed)
- Per-file limit parameter (applied to all files)
- Returns file content, hash, and error status for each file

COMPARISON:
- read_multiple_files: Batch reads, single tool call
- read: Single file, sequential calls

EXAMPLE:
{
  "paths": ["/config.json", "/README.md", "/src/main.rs"],
  "limit": 100  // Optional: limit per file (default: 500)
}

RETURNS:
Array of results, one per file, with:
- success: true/false
- path: file path
- content: file content (if success)
- hash: content hash (if success)
- error: error message (if failed)
- total_lines, truncated, offset: metadata (if success)"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of file paths to read"
                },
                "limit": {
                    "type": ["integer", "string", "null"],
                    "description": "Optional maximum lines per file (default: 500). Accepts integer or string (e.g., 100 or '100')."
                }
            },
            "required": ["paths"],
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
        let args: ReadMultipleFilesArgs = serde_json::from_value(args)?;

        if args.paths.is_empty() {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "paths".to_string(),
                message: "Paths array cannot be empty".to_string(),
            }));
        }

        if args.paths.len() > 50 {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "paths".to_string(),
                message: "Cannot read more than 50 files at once".to_string(),
            }));
        }

        // Normalize all paths
        let paths: Vec<String> = args.paths.into_iter()
            .map(|p| super::normalize_path(&p))
            .collect();

        // Read all files (sequentially due to DB connection constraints)
        let mut results: Vec<ReadFileResult> = Vec::new();

        for path in &paths {
            let result = read_single_file(
                workspace_id,
                path.clone(),
                args.limit,
                conn,
                storage
            ).await;

            match result {
                Ok(file_result) => results.push(file_result),
                Err(e) => results.push(ReadFileResult {
                    path: path.clone(),
                    success: false,
                    content: None,
                    hash: None,
                    error: Some(e.to_string()),
                    total_lines: None,
                    truncated: None,
                }),
            }
        }

        let response = ReadMultipleFilesResult {
            files: results,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(response)?,
            error: None,
        })
    }
}

/// Reads a single file, returning a ReadFileResult
async fn read_single_file(
    workspace_id: Uuid,
    path: String,
    limit: Option<usize>,
    conn: &mut DbConn,
    storage: &FileStorageService,
) -> Result<ReadFileResult> {
    let file = file_queries::get_file_by_path(conn, workspace_id, &path).await?
        .ok_or_else(|| Error::NotFound(format!("File not found: {}", path)))?;

    if matches!(file.file_type, crate::models::files::FileType::Folder) {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "path".to_string(),
            message: "Cannot read a folder".to_string(),
        }));
    }

    let file_with_content = files::get_file_with_content(conn, storage, file.id).await?;
    let latest_version = file_queries::get_latest_version(conn, file.id).await?;

    // Extract content
    let (content, total_lines, truncated) = match &file_with_content.content {
        Value::String(s) => {
            let effective_limit = limit.unwrap_or(500);
            let lines: Vec<&str> = s.lines().collect();
            let total = lines.len();
            let was_truncated = total > effective_limit;

            let sliced_content = if was_truncated {
                lines.iter().take(effective_limit).cloned().collect::<Vec<_>>().join("\n")
            } else {
                s.clone()
            };

            (Value::String(sliced_content), Some(total), Some(was_truncated))
        }
        other => (other.clone(), None, None),
    };

    Ok(ReadFileResult {
        path,
        success: true,
        content: Some(content),
        hash: Some(latest_version.hash),
        error: None,
        total_lines,
        truncated,
    })
}
