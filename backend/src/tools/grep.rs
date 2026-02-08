use crate::{DbConn, error::{Error, Result}};
use crate::models::requests::{ToolResponse, GrepArgs, GrepMatch, GrepResult};
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use std::process::Command as StdCommand;
use tokio::process::Command as TokioCommand;
use std::path::Path;
use std::collections::HashMap;
use super::{Tool, ToolConfig};

/// Grep tool for searching file contents using external binaries
///
/// Uses ripgrep (rg) if available, falls back to grep.
/// Searches for a regex pattern across all document files in a workspace.
///
/// # GOOD Examples
///
/// ```text
/// // Simple pattern search (case-insensitive by default)
/// {"pattern": "function_name"}
///
/// // Search only in specific file types
/// {"pattern": "TODO", "path_pattern": "*.rs"}
///
/// // Case-sensitive search for exact match
/// {"pattern": "Config", "case_sensitive": true}
///
/// // Search in multiple file types with wildcard
/// {"pattern": "import.*React", "path_pattern": "*.{ts,tsx}"}
///
/// // Regex pattern search
/// {"pattern": "const\\s+[A-Z][a-zA-Z]*\\s*="}
///
/// // Search for error handling patterns
/// {"pattern": "\\?\\..*\\.unwrap\\(\\)", "path_pattern": "*.rs"}
/// ```
///
/// # BAD Examples
///
/// ```text
/// // ❌ Missing 'pattern' field (required)
/// {"path_pattern": "*.rs"}
///
/// // ❌ Passing null instead of a JSON object
/// null
///
/// // ❌ Missing quotes around pattern (invalid JSON)
/// {pattern: function_name}
///
/// // ❌ Empty pattern (will not match anything meaningful)
/// {"pattern": ""}
///
/// // ❌ Using read tool instead of grep for searching
/// // BAD: Read multiple files one by one
/// read("/src/file1.rs")
/// read("/src/file2.rs")
/// read("/src/file3.rs")
/// // GOOD: Use grep to search all files at once
/// {"pattern": "search_term", "path_pattern": "*.rs"}
/// ```
///
/// # Performance Notes
///
/// - **ALWAYS use grep instead of read** when searching for patterns across multiple files
/// - Grep is 10-100x faster than reading files individually
/// - Use `path_pattern` to limit search to relevant file types
/// - Maximum 1000 matches returned to prevent large responses
/// - Case-insensitive by default (set `case_sensitive: true` for exact matches)
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Searches for a regex pattern across all document files in a workspace using ripgrep or grep. Pattern is required. Optional path_pattern filters results (supports * wildcards). Set case_sensitive to false (default) for case-insensitive search. Returns matching file paths with context. Use for discovering code patterns or finding specific content."
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "path_pattern": {"type": ["string", "null"]},
                "case_sensitive": {
                    "type": ["boolean", "null"],
                    "description": "Must be JSON boolean (true/false), not string ('true'/'false')"
                }
            },
            "required": ["pattern"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let grep_args: GrepArgs = serde_json::from_value(args)?;

        // Search only in the 'latest' directory (working tree)
        let search_path = storage.get_workspace_path(workspace_id);

        // Check if search directory exists
        if !search_path.exists() {
            return Ok(ToolResponse {
                success: true,
                result: serde_json::to_value(GrepResult { matches: Vec::new() })?,
                error: None,
            });
        }

        // Build command (tries ripgrep, then grep)
        let mut cmd = build_grep_command(
            &grep_args.pattern,
            grep_args.path_pattern.as_deref(),
            grep_args.case_sensitive.unwrap_or(false),
            &search_path,
        )?;

        // Detect if we're using ripgrep (by checking the command)
        let program = cmd.as_std().get_program().to_string_lossy();
        let is_ripgrep = program.contains("rg");

        tracing::debug!("Using grep command: {}, is_ripgrep: {}", program, is_ripgrep);

        // Execute command
        let output = cmd.output().await.map_err(|e| {
            Error::Internal(format!("Failed to execute grep command: {}", e))
        })?;

        // Handle exit codes
        // 0 = matches found
        // 1 = no matches found (successful search)
        // >1 = error
        if !output.status.success() {
            let code = output.status.code().unwrap_or(2);
            if code == 1 {
                // No matches - return success with empty list
                return Ok(ToolResponse {
                    success: true,
                    result: serde_json::to_value(GrepResult { matches: Vec::new() })?,
                    error: None,
                });
            }

            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Grep command failed (code {}): {}", code, stderr);
            return Ok(ToolResponse {
                success: false,
                result: Value::Null,
                error: Some(format!("Grep command failed: {}", stderr)),
            });
        }

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::debug!("Grep stdout length: {} bytes", stdout.len());

        let mut matches = Vec::new();
        const MAX_MATCHES: usize = 1000;

        if is_ripgrep {
            // Use JSON parser for ripgrep
            tracing::debug!("Parsing ripgrep JSON output");
            let mut file_path_cache = HashMap::new();
            for line in stdout.lines() {
                if matches.len() >= MAX_MATCHES {
                    break;
                }

                if let Some(grep_match) = parse_json_grep_output(line, &search_path, &mut file_path_cache) {
                    matches.push(grep_match);
                }
            }
        } else {
            // Use plain text parser for grep
            tracing::debug!("Parsing grep plain text output");
            for line in stdout.lines() {
                if matches.len() >= MAX_MATCHES {
                    break;
                }

                if let Some(grep_match) = parse_grep_output(line, &search_path) {
                    matches.push(grep_match);
                }
            }
        }

        tracing::debug!("Parsed {} matches", matches.len());

        // Sort matches by path, then line number for deterministic output
        matches.sort_by(|a, b| {
            a.path.cmp(&b.path).then_with(|| a.line_number.cmp(&b.line_number))
        });

        let result = GrepResult { matches };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

/// Builds the appropriate grep command (ripgrep or grep)
fn build_grep_command(
    pattern: &str,
    path_pattern: Option<&str>,
    case_sensitive: bool,
    workspace_path: &Path,
) -> Result<TokioCommand> {
    // Try ripgrep first
    if StdCommand::new("rg")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some()
    {
        tracing::debug!("Using ripgrep for search");
        return build_ripgrep_command(pattern, path_pattern, case_sensitive, workspace_path);
    }

    // Fallback to grep
    if StdCommand::new("grep")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some()
    {
        tracing::debug!("Using grep for search");
        return build_standard_grep_command(pattern, path_pattern, case_sensitive, workspace_path);
    }

    Err(Error::Internal(
        "Neither rg nor grep found on system".to_string()
    ))
}

/// Builds a ripgrep command with JSON output for robust parsing
fn build_ripgrep_command(
    pattern: &str,
    path_pattern: Option<&str>,
    case_sensitive: bool,
    workspace_path: &Path,
) -> Result<TokioCommand> {
    let mut cmd = TokioCommand::new("rg");
    cmd.arg(pattern)
       .arg(workspace_path);

    // Add glob pattern if provided
    if let Some(path_pattern) = path_pattern {
        cmd.arg("--glob").arg(path_pattern);
    }

    // Case sensitivity
    if case_sensitive {
        cmd.arg("--case-sensitive");
    } else {
        cmd.arg("--ignore-case");
    }

    // Use JSON output for machine-readable parsing
    cmd.args(["--json", "--line-number"]);

    Ok(cmd)
}

/// Builds a standard grep command
fn build_standard_grep_command(
    pattern: &str,
    path_pattern: Option<&str>,
    case_sensitive: bool,
    workspace_path: &Path,
) -> Result<TokioCommand> {
    let mut cmd = TokioCommand::new("grep");
    cmd.arg("-R")  // Recursive
       .arg("-n")  // Line numbers
       .arg("-H")  // Always show filename
       .arg(pattern);

    if !case_sensitive {
        cmd.arg("-i");  // Case insensitive
    }

    // Add path filter if provided
    if let Some(path_pattern) = path_pattern {
        cmd.arg("--include").arg(path_pattern);
    }

    cmd.arg(workspace_path);

    Ok(cmd)
}

/// Parses grep output line into GrepMatch
/// Expected format: "path:line_number:line_content"
fn parse_grep_output(line: &str, workspace_path: &Path) -> Option<GrepMatch> {
    // Split on ':' but only into 3 parts max (path:line:content)
    let parts: Vec<&str> = line.splitn(3, ':').collect();
    if parts.len() < 3 {
        return None;
    }

    let full_path = parts[0].trim();
    let line_number: i32 = parts[1].parse().ok()?;
    let line_text = parts[2].to_string();

    // Convert absolute path to relative path from workspace
    let relative_path = Path::new(full_path)
        .strip_prefix(workspace_path)
        .map(|p| p.to_str().unwrap_or(full_path))
        .unwrap_or(full_path);

    // Add leading "/" to match workspace path convention
    let path_with_slash = format!("/{}", relative_path);

    Some(GrepMatch {
        path: path_with_slash,
        line_number,
        line_text,
    })
}

/// Ripgrep JSON event types
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RipgrepEvent {
    Begin { data: BeginData },
    Match { data: MatchData },
    End {
        #[allow(dead_code)]
        data: EndData
    },
    Summary {
        #[allow(dead_code)]
        data: SummaryData
    },
}

#[derive(Debug, serde::Deserialize)]
struct BeginData {
    path: PathField,
}

#[derive(Debug, serde::Deserialize)]
struct MatchData {
    path: PathField,
    lines: LinesField,
    line_number: Option<u64>,
    #[allow(dead_code)]
    submatches: Vec<Submatch>,
}

#[derive(Debug, serde::Deserialize)]
struct EndData {
    #[allow(dead_code)]
    path: PathField,
    #[allow(dead_code)]
    binary_offset: Option<serde_json::Value>,
    #[allow(dead_code)]
    stats: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct SummaryData {
    #[allow(dead_code)]
    elapsed_total: Option<serde_json::Value>,
    #[allow(dead_code)]
    stats: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct PathField {
    text: String,
}

#[derive(Debug, serde::Deserialize)]
struct LinesField {
    text: String,
}

#[derive(Debug, serde::Deserialize)]
struct Submatch {
    #[serde(rename = "match")]
    #[allow(dead_code)]
    text: Value,
    #[allow(dead_code)]
    start: usize,
    #[allow(dead_code)]
    end: usize,
}

/// Parses ripgrep JSON output (line-delimited JSON)
/// Returns None for non-match events (begin/end/summary)
fn parse_json_grep_output(line: &str, workspace_path: &Path, file_path_cache: &mut HashMap<String, String>) -> Option<GrepMatch> {
    let event: RipgrepEvent = serde_json::from_str(line).ok()?;

    match event {
        RipgrepEvent::Begin { data } => {
            // Cache the file path for subsequent matches
            let full_path = data.path.text;
            let relative_path = Path::new(&full_path)
                .strip_prefix(workspace_path)
                .map(|p| p.to_str().unwrap_or(&full_path))
                .unwrap_or(&full_path);
            let path_with_slash = format!("/{}", relative_path);
            tracing::trace!("JSON begin event: {} -> {}", full_path, path_with_slash);
            file_path_cache.insert(full_path.clone(), path_with_slash);
            None
        }
        RipgrepEvent::Match { data } => {
            // Get the path from cache or resolve it
            let full_path = data.path.text;
            let relative_path = file_path_cache.get(&full_path)
                .cloned()
                .or_else(|| {
                    Path::new(&full_path)
                        .strip_prefix(workspace_path)
                        .map(|p| format!("/{}", p.to_str().unwrap_or(&full_path)))
                        .ok()
                })?;

            // Ripgrep includes newlines in the output, strip them to match grep behavior
            let line_text = data.lines.text.trim_end().to_string();
            let line_number = data.line_number.map(|n| n as i32).unwrap_or(0);

            tracing::trace!("JSON match: {}:{}: {}", relative_path, line_number, line_text);

            Some(GrepMatch {
                path: relative_path,
                line_number,
                line_text,
            })
        }
        RipgrepEvent::End { .. } => {
            tracing::trace!("JSON end event");
            None
        }
        RipgrepEvent::Summary { .. } => {
            tracing::trace!("JSON summary event");
            None
        }
    }
}
