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

/// State for tracking context lines across ripgrep JSON events
struct ContextTracker {
    before_context: Vec<String>,
    after_context: Vec<String>,
    pending_match: Option<GrepMatch>,
}

impl ContextTracker {
    fn new() -> Self {
        Self {
            before_context: Vec::new(),
            after_context: Vec::new(),
            pending_match: None,
        }
    }

    fn add_before_context(&mut self, line: String) {
        self.before_context.push(line);
    }

    fn add_after_context(&mut self, line: String) {
        self.after_context.push(line);
    }

    fn set_match(&mut self, grep_match: GrepMatch) {
        // Attach any accumulated before-context to this match
        let mut match_with_context = grep_match;
        if !self.before_context.is_empty() {
            match_with_context.before_context = Some(std::mem::take(&mut self.before_context));
        }
        self.pending_match = Some(match_with_context);
    }

    fn finalize_match(&mut self) -> Option<GrepMatch> {
        if let Some(mut match_with_context) = self.pending_match.take() {
            // Attach any accumulated after-context to this match
            if !self.after_context.is_empty() {
                match_with_context.after_context = Some(std::mem::take(&mut self.after_context));
            }
            Some(match_with_context)
        } else {
            None
        }
    }

    fn has_pending_match(&self) -> bool {
        self.pending_match.is_some()
    }
}

/// Checks if a file path matches a glob pattern
/// Supports basic wildcards: * and **
fn path_matches_glob(file_path: &str, pattern: &str) -> bool {
    let file_path = file_path.strip_prefix('/').unwrap_or(file_path);
    let pattern = pattern.strip_prefix('/').unwrap_or(pattern);

    // Handle common glob patterns
    if pattern.contains("**") {
        // ** matches anything including slashes
        let base = pattern.split("**").next().unwrap_or("");
        if base.is_empty() {
            return true;
        }
        return file_path.starts_with(base.trim_end_matches('/'));
    }

    if pattern.contains('*') {
        // Split by * and check if path matches
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            // Pattern like "scripts/*.rs"
            let (prefix, suffix) = (parts[0], parts[1]);
            if !suffix.is_empty() {
                // Check suffix match (e.g., .rs extension)
                file_path.starts_with(prefix.trim_end_matches('/'))
                    && file_path.ends_with(suffix)
            } else {
                // Pattern like "scripts/*"
                file_path.starts_with(prefix.trim_end_matches('/'))
            }
        } else {
            // Multiple wildcards, do substring match
            file_path.contains(&pattern.replace('*', ""))
        }
    } else {
        // No wildcards - exact directory match or prefix match
        file_path == pattern
            || file_path.starts_with(&format!("{}/", pattern))
            || file_path.starts_with(&format!("{}", pattern.trim_end_matches('/')))
    }
}

/// Grep tool for searching file contents using external binaries
///
/// Uses ripgrep (rg) if available, falls back to grep.
/// Searches for a regex pattern across all document files in a workspace.
///
/// # path_pattern Behavior
///
/// The `path_pattern` parameter is **relative to the workspace root**.
/// Leading slashes are automatically stripped for convenience.
///
/// Examples:
/// - `"scripts/*"` - matches all files in the scripts folder
/// - `"/scripts/*"` - same as above (leading slash is stripped)
/// - `"*.rs"` - matches all .rs files anywhere in workspace
/// - `"/src/**/*.rs"` - same as "src/**/*.rs"
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
/// // Search in scripts folder (leading slash is optional)
/// {"pattern": "main", "path_pattern": "/scripts/*"}
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
        r#"Searches for a regex pattern across all document files in a workspace using ripgrep or grep. Pattern is required. Optional path_pattern filters results (supports * wildcards). Set case_sensitive to false (default) for case-insensitive search.

CONTEXT PARAMETERS (optional):
- before_context: Number of lines to show before each match
- after_context: Number of lines to show after each match
- context: Shorthand for both before and after context (e.g., context=3 shows 3 lines before and after)

Returns matching file paths with line numbers and context lines. Use for discovering code patterns or finding specific content."#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "path_pattern": {"type": ["string", "null"]},
                "case_sensitive": {
                    "type": ["boolean", "string", "null"],
                    "description": "Accepts JSON boolean (true/false) or string representations ('true', 'True', 'false', 'False', 'TRUE', 'FALSE'). Defaults to false (case-insensitive) if not provided."
                },
                "before_context": {
                    "type": ["integer", "null"],
                    "description": "Number of lines to show before each match (default: 0)"
                },
                "after_context": {
                    "type": ["integer", "null"],
                    "description": "Number of lines to show after each match (default: 0)"
                },
                "context": {
                    "type": ["integer", "null"],
                    "description": "Shorthand for before_context and after_context combined"
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

        // Security: Validate path_pattern doesn't attempt to escape workspace
        if let Some(ref path_pattern) = grep_args.path_pattern {
            let normalized = path_pattern.strip_prefix('/').unwrap_or(path_pattern);
            if normalized.contains("..") {
                return Ok(ToolResponse {
                    success: false,
                    result: Value::Null,
                    error: Some("path_pattern cannot contain '..' (parent directory reference)".to_string()),
                });
            }
        }

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
        // Determine context parameters
        let before = grep_args.context.unwrap_or(0);
        let after = grep_args.context.unwrap_or(0);
        let before_context = grep_args.before_context.unwrap_or(before);
        let after_context = grep_args.after_context.unwrap_or(after);

        let mut cmd = build_grep_command(
            &grep_args.pattern,
            grep_args.path_pattern.as_deref(),
            grep_args.case_sensitive.unwrap_or(false),
            before_context,
            after_context,
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

        // Get normalized path_pattern for filtering (needed for grep fallback)
        let path_pattern_filter = grep_args.path_pattern.as_deref();

        if is_ripgrep {
            // Use JSON parser for ripgrep
            tracing::debug!("Parsing ripgrep JSON output");
            let mut file_path_cache = HashMap::new();
            let mut context_tracker = ContextTracker::new();
            for line in stdout.lines() {
                if matches.len() >= MAX_MATCHES {
                    break;
                }

                if let Some(grep_match) = parse_json_grep_output(line, &search_path, &mut file_path_cache, &mut context_tracker) {
                    matches.push(grep_match);
                }
            }
            // Don't forget to finalize the last match if there is one
            if let Some(final_match) = context_tracker.finalize_match() {
                matches.push(final_match);
            }
        } else {
            // Use plain text parser for grep
            tracing::debug!("Parsing grep plain text output");
            for line in stdout.lines() {
                if matches.len() >= MAX_MATCHES {
                    break;
                }

                if let Some(grep_match) = parse_grep_output(line, &search_path) {
                    // Filter by path_pattern if provided (needed for grep fallback)
                    if let Some(pattern) = path_pattern_filter {
                        if path_matches_glob(&grep_match.path, pattern) {
                            matches.push(grep_match);
                        }
                    } else {
                        matches.push(grep_match);
                    }
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
    before_context: usize,
    after_context: usize,
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
        return build_ripgrep_command(pattern, path_pattern, case_sensitive, before_context, after_context, workspace_path);
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
        return build_standard_grep_command(pattern, path_pattern, case_sensitive, before_context, after_context, workspace_path);
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
    before_context: usize,
    after_context: usize,
    workspace_path: &Path,
) -> Result<TokioCommand> {
    let mut cmd = TokioCommand::new("rg");

    // Set current directory to workspace_path so glob patterns work correctly
    cmd.current_dir(workspace_path);

    // Search in current directory (.)
    cmd.arg(pattern).arg(".");

    // Add glob pattern if provided
    // Strip leading slashes to make pattern relative to workspace root
    if let Some(path_pattern) = path_pattern {
        let normalized_pattern = path_pattern.strip_prefix('/').unwrap_or(path_pattern);
        cmd.arg("--glob").arg(normalized_pattern);
    }

    // Case sensitivity
    if case_sensitive {
        cmd.arg("--case-sensitive");
    } else {
        cmd.arg("--ignore-case");
    }

    // Add context if requested
    if before_context > 0 {
        cmd.arg("--before-context").arg(before_context.to_string());
    }
    if after_context > 0 {
        cmd.arg("--after-context").arg(after_context.to_string());
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
    before_context: usize,
    after_context: usize,
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

    // Add context if requested
    if before_context > 0 {
        cmd.arg("-B").arg(before_context.to_string());
    }
    if after_context > 0 {
        cmd.arg("-A").arg(after_context.to_string());
    }

    // Note: grep's --include only supports filename patterns, not path patterns
    // For path-based filtering (e.g., "scripts/*.rs"), we filter during parsing
    // For simple filename patterns (e.g., "*.rs"), we can use --include
    if let Some(path_pattern) = path_pattern {
        let normalized_pattern = path_pattern.strip_prefix('/').unwrap_or(path_pattern);
        // Only use --include if it's a simple filename pattern (no directory path)
        if !normalized_pattern.contains('/') {
            cmd.arg("--include").arg(normalized_pattern);
        }
    }

    // Set current directory to workspace_path
    cmd.current_dir(workspace_path);
    cmd.arg(".");

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

    // Convert to relative path from workspace
    // First try stripping workspace_path (for absolute paths)
    let relative_path = if Path::new(full_path).is_absolute() {
        Path::new(full_path)
            .strip_prefix(workspace_path)
            .map(|p| p.to_str().unwrap_or(full_path))
            .unwrap_or(full_path)
    } else {
        // For relative paths (from current_dir), just strip leading ./
        full_path.strip_prefix("./").unwrap_or(full_path)
    };

    // Add leading "/" to match workspace path convention
    let path_with_slash = format!("/{}", relative_path);

    Some(GrepMatch {
        path: path_with_slash,
        line_number,
        line_text,
        before_context: None,  // Context parsing for plain grep is complex, skip for now
        after_context: None,
    })
}

/// Ripgrep JSON event types
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RipgrepEvent {
    Begin { data: BeginData },
    Match { data: MatchData },
    Context { data: ContextData },
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
struct ContextData {
    #[allow(dead_code)]
    path: PathField,
    lines: LinesField,
    #[allow(dead_code)]
    line_number: u64,
    #[allow(dead_code)]
    absolute_offset: u64,
    #[allow(dead_code)]
    submatches: Vec<serde_json::Value>,
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
/// Returns None for non-match events (begin/end/summary/context)
/// Returns Some(match) when a match is finalized (after seeing its after-context or next event)
fn parse_json_grep_output(
    line: &str,
    workspace_path: &Path,
    file_path_cache: &mut HashMap<String, String>,
    context_tracker: &mut ContextTracker,
) -> Option<GrepMatch> {
    let event: RipgrepEvent = serde_json::from_str(line).ok()?;

    match event {
        RipgrepEvent::Begin { data } => {
            // Cache the file path for subsequent matches
            let full_path = data.path.text;
            // Convert to relative path
            let relative_path = if Path::new(&full_path).is_absolute() {
                Path::new(&full_path)
                    .strip_prefix(workspace_path)
                    .map(|p| p.to_str().unwrap_or(&full_path))
                    .unwrap_or(&full_path)
            } else {
                // For relative paths (from current_dir), strip leading ./
                full_path.strip_prefix("./").unwrap_or(&full_path)
            };
            let path_with_slash = format!("/{}", relative_path);
            tracing::trace!("JSON begin event: {} -> {}", full_path, path_with_slash);
            file_path_cache.insert(full_path.clone(), path_with_slash);
            None
        }
        RipgrepEvent::Context { data } => {
            let line_text = data.lines.text.trim_end().to_string();

            if context_tracker.has_pending_match() {
                // This is after-context for the current match
                context_tracker.add_after_context(line_text);
            } else {
                // This is before-context for the next match
                context_tracker.add_before_context(line_text);
            }
            None
        }
        RipgrepEvent::Match { data } => {
            // Finalize previous match if there is one
            let previous_match = context_tracker.finalize_match();

            // Get the path from cache or resolve it
            let full_path = data.path.text;
            let relative_path = file_path_cache.get(&full_path)
                .cloned()
                .or_else(|| {
                    // Convert to relative path
                    let rel_path = if Path::new(&full_path).is_absolute() {
                        Path::new(&full_path)
                            .strip_prefix(workspace_path)
                            .map(|p| p.to_str().unwrap_or(&full_path))
                            .unwrap_or(&full_path)
                    } else {
                        full_path.strip_prefix("./").unwrap_or(&full_path)
                    };
                    Some(format!("/{}", rel_path))
                })?;

            // Ripgrep includes newlines in the output, strip them to match grep behavior
            let line_text = data.lines.text.trim_end().to_string();
            let line_number = data.line_number.map(|n| n as i32).unwrap_or(0);

            tracing::trace!("JSON match: {}:{}: {}", relative_path, line_number, line_text);

            // Set this as the pending match (will be finalized when we see the next event)
            context_tracker.set_match(GrepMatch {
                path: relative_path,
                line_number,
                line_text,
                before_context: None,
                after_context: None,
            });

            // Return the previous match (now finalized)
            previous_match
        }
        RipgrepEvent::End { .. } => {
            tracing::trace!("JSON end event");
            // Finalize any pending match when we reach end of file
            context_tracker.finalize_match()
        }
        RipgrepEvent::Summary { .. } => {
            tracing::trace!("JSON summary event");
            None
        }
    }
}
