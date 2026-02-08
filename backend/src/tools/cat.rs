use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, CatArgs, CatResult, CatFileEntry}, queries::files as file_queries, services::files};
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Cat tool for concatenating multiple files
///
/// Displays multiple files sequentially, like Unix cat.
pub struct CatTool;

#[async_trait]
impl Tool for CatTool {
    fn name(&self) -> &'static str {
        "cat"
    }

    fn description(&self) -> &'static str {
        r#"Concatenates and displays multiple files. Useful for reviewing logs, comparing configs, or multi-file analysis.

OPTIONS:
- show_headers: Add filename headers before each file's content (default: false)
- number_lines: Add line numbers to output (default: false)

FEATURES:
- Concatenates multiple files in a single operation
- Optional filename headers for clarity
- Optional line numbers for reference
- Returns aggregated content with per-file metadata

COMPARISON:
- cat: Multiple files, concatenated output
- read_multiple_files: Multiple files, structured JSON output
- read: Single file only

EXAMPLES:
{"paths": ["/config.json", "/.env.example"]} - Concatenate configs
{"paths": ["/logs/*.log"], "show_headers": true} - Show logs with headers
{"paths": ["/file1.txt", "/file2.txt"], "number_lines": true} - With line numbers

RETURNS:
- content: Concatenated content of all files
- files: Array of per-file entries with path, content, line_count
- Errors for individual files are shown in their entries"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of file paths to concatenate"
                },
                "show_headers": {
                    "type": ["boolean", "null"],
                    "description": "Add filename headers before each file (default: false)"
                },
                "number_lines": {
                    "type": ["boolean", "null"],
                    "description": "Add line numbers to output (default: false)"
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
        let args: CatArgs = serde_json::from_value(args)?;

        if args.paths.is_empty() {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "paths".to_string(),
                message: "Paths array cannot be empty".to_string(),
            }));
        }

        if args.paths.len() > 20 {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "paths".to_string(),
                message: "Cannot concatenate more than 20 files at once".to_string(),
            }));
        }

        let show_headers = args.show_headers.unwrap_or(false);
        let number_lines = args.number_lines.unwrap_or(false);

        // Normalize all paths
        let paths: Vec<String> = args.paths.into_iter()
            .map(|p| super::normalize_path(&p))
            .collect();

        // Process each file
        let mut file_entries: Vec<CatFileEntry> = Vec::new();
        let mut concatenated_content = String::new();

        for (index, path) in paths.iter().enumerate() {
            // Add separator between files (unless first file)
            if index > 0 && !concatenated_content.is_empty() {
                concatenated_content.push('\n');
            }

            // Add header if requested
            if show_headers {
                concatenated_content.push_str(&format!("==> {} <==\n", path));
            }

            // Try to read the file
            match read_single_file(conn, storage, workspace_id, path.clone()).await {
                Ok(content) => {
                    let line_count = content.lines().count();

                    // Add line numbers if requested
                    let numbered_content = if number_lines {
                        content.lines()
                            .enumerate()
                            .map(|(i, line)| format!("{:6}\t{}", i + 1, line))
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        content.clone()
                    };

                    file_entries.push(CatFileEntry {
                        path: path.clone(),
                        content: numbered_content.clone(),
                        line_count,
                    });

                    if !concatenated_content.is_empty() && !numbered_content.ends_with('\n') {
                        concatenated_content.push('\n');
                    }
                    concatenated_content.push_str(&numbered_content);
                }
                Err(e) => {
                    let error_msg = format!("Error reading {}: {}", path, e);
                    file_entries.push(CatFileEntry {
                        path: path.clone(),
                        content: error_msg.clone(),
                        line_count: 0,
                    });
                    concatenated_content.push_str(&error_msg);
                    concatenated_content.push('\n');
                }
            }
        }

        let result = CatResult {
            content: concatenated_content,
            files: file_entries,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

/// Reads a single file, returning its content as a String
async fn read_single_file(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    path: String,
) -> Result<String> {
    let file = file_queries::get_file_by_path(conn, workspace_id, &path).await?
        .ok_or_else(|| Error::NotFound(format!("File not found: {}", path)))?;

    if matches!(file.file_type, crate::models::files::FileType::Folder) {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "path".to_string(),
            message: "Cannot read a folder".to_string(),
        }));
    }

    let file_with_content = files::get_file_with_content(conn, storage, file.id).await?;

    // Extract content as String
    let content = match &file_with_content.content {
        Value::String(s) => s.clone(),
        other => {
            if let Some(s) = other.as_str() {
                s.to_string()
            } else {
                serde_json::to_string(other)?
            }
        }
    };

    Ok(content)
}
