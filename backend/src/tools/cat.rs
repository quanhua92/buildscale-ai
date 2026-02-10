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

/// Processes content with all transformations applied
fn process_content(
    content: &str,
    number_lines: bool,
    squeeze_blank: bool,
    line_config: &LineConfig,
    starting_line_number: usize,
) -> String {
    let mut result = Vec::new();
    let mut line_number = starting_line_number;
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

LINE RANGE FILTERING:
- offset: Starting line position (positive=from start, negative=from end)
- limit: Maximum number of lines to read per file
- Line numbers reflect actual file position when using offset

NUMBERING OPTIONS:
- number_lines: Number all lines (smart numbering with offset)

DISPLAY OPTIONS:
- show_headers: Add filename headers before each file

FEATURES:
- Concatenates multiple files in a single operation
- Special character display for debugging (tabs, whitespace)
- Line range filtering for targeted debugging
- Smart line numbering reflects actual file position
- Squeezes excessive blank lines for readability
- Returns aggregated content with per-file metadata

COMPARISON:
- cat: Multiple files, special character display, line range filtering, formatting options
- read: Single file, pagination, scroll mode, cursor navigation

EXAMPLES:
{"paths": ["/config.json", "/.env"]} - Basic concatenation
{"paths": ["/file.txt"], "show_ends": true} - Show trailing whitespace
{"paths": ["/code.rs"], "show_tabs": true} - Reveal tab characters
{"paths": ["/log.txt"], "offset": -100, "limit": 50} - Last 100 lines, max 50
{"paths": ["/data.txt"], "offset": 100, "limit": 50, "show_ends": true} - Lines 100-149 with trailing whitespace shown
{"paths": ["/file.txt"], "offset": 100, "limit": 50, "number_lines": true, "show_tabs": true} - Lines 101-150 numbered, tabs shown

RETURNS:
- content: Concatenated and formatted content of all files
- files: Array of per-file entries with path, content, line_count, offset, limit, total_lines
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
                    "description": "Add line numbers to all lines (default: false). Line numbers reflect actual file position when using offset."
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
                },
                "offset": {
                    "type": ["integer", "string", "null"],
                    "description": "Starting line position (default: 0). Accepts integer or string (e.g., 100 or '100'). Positive values start from beginning (e.g., 100 = line 100+). Negative values read from end (e.g., -50 = last 50 lines). Line numbers reflect actual position."
                },
                "limit": {
                    "type": ["integer", "string", "null"],
                    "description": "Maximum number of lines to read per file (default: unlimited). Accepts integer or string (e.g., 50 or '50'). Use with offset to read specific ranges."
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

        // Extract offset/limit
        let offset = args.offset.unwrap_or(0);
        let limit = args.limit.unwrap_or(usize::MAX);  // No limit if not specified

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
                    // Apply offset/limit slicing
                    let (sliced_content, total_lines, _) = if offset < 0 {
                        // Negative offset: read from end
                        let lines_from_end = offset.abs() as usize;
                        slice_content_from_end(&content, lines_from_end, limit)
                    } else {
                        // Positive offset: read from specific line
                        slice_content_by_lines(&content, offset as usize, limit)
                    };

                    // Calculate starting line number for smart numbering
                    let starting_line_number = if offset < 0 {
                        total_lines.saturating_sub(offset.abs() as usize)
                    } else {
                        offset as usize
                    };

                    // Use process_content with smart line numbering
                    let processed_content = process_content(
                        &sliced_content,
                        number_lines,
                        squeeze_blank,
                        &line_config,
                        starting_line_number + 1,  // +1 for 1-based line numbering
                    );

                    let sliced_line_count = sliced_content.lines().count();

                    file_entries.push(CatFileEntry {
                        path: path.clone(),
                        content: processed_content.clone(),
                        line_count: sliced_line_count,
                        offset: Some(if offset < 0 {
                            total_lines.saturating_sub(offset.abs() as usize)
                        } else {
                            offset as usize
                        }),
                        limit: Some(limit),
                        total_lines: Some(total_lines),
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
                        offset: None,
                        limit: None,
                        total_lines: None,
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
