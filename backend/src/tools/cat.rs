use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, CatArgs, CatResult, CatFileEntry}, queries::files as file_queries, services::files};
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Converts special characters to visible representations
mod escape {
    /// Show tab as ^I
    pub fn show_tabs(line: &str) -> String {
        line.replace('\t', "^I")
    }

    /// Show $ at end of line
    pub fn show_ends(line: &str) -> String {
        format!("{}$", line)
    }
}

/// Configuration for line processing
#[derive(Default)]
struct LineConfig {
    show_ends: bool,
    show_tabs: bool,
}

/// Processes a single line with transformations
fn process_line(line: &str, config: &LineConfig) -> String {
    let mut processed = line.to_string();

    if config.show_tabs {
        processed = escape::show_tabs(&processed);
    }

    if config.show_ends {
        processed = escape::show_ends(&processed);
    }

    processed
}

/// Processes content with all transformations applied
fn process_content(
    content: &str,
    number_lines: bool,
    squeeze_blank: bool,
    line_config: &LineConfig,
) -> String {
    let mut result = Vec::new();
    let mut line_number = 1;
    let mut prev_was_blank = false;

    for line in content.lines() {
        let is_blank = line.is_empty();

        // Handle squeeze_blank
        if squeeze_blank && is_blank && prev_was_blank {
            continue;
        }
        prev_was_blank = is_blank;

        // Process line content
        let processed_line = process_line(line, line_config);

        // Add line numbers
        let final_line = if number_lines {
            format!("{:6}\t{}", line_number, processed_line)
        } else {
            processed_line
        };

        line_number += 1;
        result.push(final_line);
    }

    result.join("\n")
}

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
        r#"Concatenates and displays multiple files with Unix-style formatting options.

SPECIAL CHARACTERS OPTIONS:
- show_ends: Display $ at end of each line (reveals trailing whitespace)
- show_tabs: Display tabs as ^I (distinguish tabs from spaces)
- squeeze_blank: Suppress repeated empty lines (squeeze multiple \n into one)

NUMBERING OPTIONS:
- number_lines: Number all lines (1, 2, 3, ...)

DISPLAY OPTIONS:
- show_headers: Add filename headers before each file

FEATURES:
- Concatenates multiple files in a single operation
- Special character display for debugging (tabs, whitespace)
- Optional line numbering
- Squeezes excessive blank lines for readability
- Returns aggregated content with per-file metadata

COMPARISON:
- cat: Multiple files, special character display, formatting options
- read_multiple_files: Multiple files, structured JSON output
- read: Single file, pagination, scroll mode

EXAMPLES:
{"paths": ["/config.json", "/.env"]} - Basic concatenation
{"paths": ["/file.txt"], "show_ends": true} - Show trailing whitespace
{"paths": ["/code.rs"], "show_tabs": true} - Reveal tab characters
{"paths": ["/log.txt"], "squeeze_blank": true} - Clean up sparse logs
{"paths": ["/data.txt"], "show_ends": true, "show_tabs": true} - Combined

RETURNS:
- content: Concatenated and formatted content of all files
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
                    "description": "Add line numbers to all lines (default: false)"
                },
                "show_ends": {
                    "type": ["boolean", "string", "null"],
                    "description": "Display $ at end of each line to show trailing whitespace. Accepts boolean or string (e.g., true or 'true'). Default: false"
                },
                "show_tabs": {
                    "type": ["boolean", "string", "null"],
                    "description": "Display tab characters as ^I. Accepts boolean or string (e.g., true or 'true'). Default: false"
                },
                "squeeze_blank": {
                    "type": ["boolean", "string", "null"],
                    "description": "Suppress repeated empty lines (squeeze multiple \\n into one). Accepts boolean or string (e.g., true or 'true'). Default: false"
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

        // Extract all options
        let show_headers = args.show_headers.unwrap_or(false);
        let number_lines = args.number_lines.unwrap_or(false);
        let show_ends = args.show_ends.unwrap_or(false);
        let show_tabs = args.show_tabs.unwrap_or(false);
        let squeeze_blank = args.squeeze_blank.unwrap_or(false);

        // Create line processing config
        let line_config = LineConfig {
            show_ends,
            show_tabs,
        };

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

                    // Use new process_content function
                    let processed_content = process_content(
                        &content,
                        number_lines,
                        squeeze_blank,
                        &line_config,
                    );

                    file_entries.push(CatFileEntry {
                        path: path.clone(),
                        content: processed_content.clone(),
                        line_count,
                    });

                    if !concatenated_content.is_empty() && !processed_content.ends_with('\n') {
                        concatenated_content.push('\n');
                    }
                    concatenated_content.push_str(&processed_content);
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
