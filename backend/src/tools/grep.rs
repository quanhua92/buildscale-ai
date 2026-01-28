use crate::{DbConn, error::{Error, Result}, queries};
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
        conn: &mut DbConn,
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

        // Map storage paths to logical database paths
        let mut matches = map_storage_paths_to_logical_paths(conn, workspace_id, matches).await?;

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

/// Maps storage paths to logical database paths.
/// For files stored with flat storage (/{slug}), queries the database
/// to get the full logical path (e.g., /chats/chat-{id}).
/// Preserves the order of input matches.
async fn map_storage_paths_to_logical_paths(
    conn: &mut DbConn,
    workspace_id: Uuid,
    matches: Vec<GrepMatch>,
) -> Result<Vec<GrepMatch>> {
    let mut updated_matches = Vec::new();

    for m in matches {
        let storage_path = &m.path;

        // Extract slug from storage path and try to map to logical path
        if let Some(slug) = extract_slug_from_path(storage_path) {
            // Try to find file by slug in database
            if let Ok(Some(file)) = queries::files::get_file_by_slug_any_parent(conn, workspace_id, &slug).await {
                // Use logical path from database
                updated_matches.push(GrepMatch {
                    path: file.path.clone(),
                    line_number: m.line_number,
                    line_text: m.line_text,
                });
                continue;
            }
        }

        // If no mapping found, use original path
        updated_matches.push(GrepMatch {
            path: m.path.clone(),
            line_number: m.line_number,
            line_text: m.line_text,
        });
    }

    Ok(updated_matches)
}

/// Extracts slug from a storage path.
/// - "/chat-{id}" -> "chat-{id}"
/// - "/work/file.txt" -> "file.txt"
/// - "/folder/subfolder/file.md" -> "file.md"
fn extract_slug_from_path(path: &str) -> Option<String> {
    // Remove leading slash
    let without_prefix = path.strip_prefix('/')?;

    // Get the last component (filename)
    without_prefix.split('/').last().map(|s| s.to_string())
}
