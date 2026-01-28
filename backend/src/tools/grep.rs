use crate::{DbConn, error::{Error, Result}};
use crate::models::requests::{ToolResponse, GrepArgs, GrepMatch, GrepResult};
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use std::process::Command as StdCommand;
use tokio::process::Command as TokioCommand;
use std::path::Path;
use super::Tool;

/// Grep tool for searching file contents using external binaries
///
/// Uses ripgrep (rg) if available, falls back to grep.
/// Searches for a regex pattern across all document files in a workspace.
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
        serde_json::to_value(schemars::schema_for!(GrepArgs)).unwrap_or(Value::Null)
    }

    async fn execute(
        &self,
        _conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let grep_args: GrepArgs = serde_json::from_value(args)?;

        let workspace_path = storage.get_workspace_path(workspace_id);

        // Check if workspace directory exists
        if !workspace_path.exists() {
            return Ok(ToolResponse {
                success: false,
                result: Value::Null,
                error: Some(format!("Workspace directory not found: {:?}", workspace_path)),
            });
        }

        // Build command (tries ripgrep, then grep)
        let mut cmd = build_grep_command(
            &grep_args.pattern,
            grep_args.path_pattern.as_deref(),
            grep_args.case_sensitive.unwrap_or(false),
            &workspace_path,
        )?;

        // Execute command
        let output = cmd.output().await.map_err(|e| {
            Error::Internal(format!("Failed to execute grep command: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(ToolResponse {
                success: false,
                result: Value::Null,
                error: Some(format!("Grep command failed: {}", stderr)),
            });
        }

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut matches = Vec::new();
        const MAX_MATCHES: usize = 1000;

        for line in stdout.lines() {
            if matches.len() >= MAX_MATCHES {
                break;
            }

            if let Some(grep_match) = parse_grep_output(line, &workspace_path) {
                matches.push(grep_match);
            }
        }

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

/// Builds a ripgrep command
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

    // Output format: line number + filename + line content
    cmd.args(["--line-number", "--no-heading", "--with-filename"]);

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
