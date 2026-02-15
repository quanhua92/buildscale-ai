use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, ReadMultipleFilesArgs, ReadMultipleFilesResult, ReadFileResult}, queries::files as file_queries, services::files};
use crate::services::storage::FileStorageService;
use crate::tools::helpers;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Default maximum number of lines to read from each file.
const DEFAULT_READ_LIMIT: usize = 500;

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
        "Reads multiple files (max 50). Parameters: paths (array), limit (default 500). Returns per-file content, hash, error."
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
                    synced: false,
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
    // Try database lookup first
    let file = match file_queries::get_file_by_path(conn, workspace_id, &path).await? {
        Some(f) => f,
        None => {
            // Fallback: Check if file exists on disk
            match helpers::file_exists_on_disk(storage, workspace_id, &path).await {
                Ok(true) => {
                    // File exists on disk but not in database - read from disk
                    let (content, hash) = match helpers::read_file_from_disk(
                        storage,
                        workspace_id,
                        &path,
                    ).await {
                        Ok((c, h)) => (c, h),
                        Err(e) => {
                            return Ok(ReadFileResult {
                                path,
                                success: false,
                                content: None,
                                hash: None,
                                synced: false,
                                error: Some(e.to_string()),
                                total_lines: None,
                                truncated: None,
                            });
                        }
                    };

                    let effective_limit = limit.map(|l| if l == 0 { usize::MAX } else { l }).unwrap_or(DEFAULT_READ_LIMIT);
                    let lines: Vec<&str> = content.lines().collect();
                    let total = lines.len();
                    let was_truncated = total > effective_limit;

                    let sliced_content = if was_truncated {
                        lines.iter().take(effective_limit).cloned().collect::<Vec<_>>().join("\n")
                    } else {
                        content.clone()
                    };

                    return Ok(ReadFileResult {
                        path,
                        success: true,
                        content: Some(Value::String(sliced_content)),
                        hash: Some(hash),
                        synced: false,  // Filesystem-only
                        error: None,
                        total_lines: Some(total),
                        truncated: Some(was_truncated),
                    });
                }
                Ok(false) => {
                    // File not found in database or on disk
                    return Ok(ReadFileResult {
                        path: path.clone(),
                        success: false,
                        content: None,
                        hash: None,
                        synced: false,
                        error: Some(format!("File not found: {}", path)),
                        total_lines: None,
                        truncated: None,
                    });
                }
                Err(e) => {
                    // Error checking disk - return error result
                    return Ok(ReadFileResult {
                        path,
                        success: false,
                        content: None,
                        hash: None,
                        synced: false,
                        error: Some(e.to_string()),
                        total_lines: None,
                        truncated: None,
                    });
                }
            }
        }
    };

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
            let effective_limit = limit.map(|l| if l == 0 { usize::MAX } else { l }).unwrap_or(DEFAULT_READ_LIMIT);
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
        synced: true,  // Database entry
        error: None,
        total_lines,
        truncated,
    })
}
