use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, GlobArgs, GlobResult, GlobMatch}, queries::files};
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use std::process::Command as StdCommand;
use tokio::process::Command as TokioCommand;
use std::path::Path;
use super::{Tool, ToolConfig};

/// Glob tool for pattern-based file search using ripgrep
///
/// Finds files matching glob patterns like `*.rs`, `**/*.md`, `/src/**/*.rs`.
/// Uses ripgrep's --files option for efficient file discovery.
///
/// # Security: Workspace Isolation
///
/// This tool ensures workspace isolation by:
/// 1. Setting ripgrep's current directory to the workspace root
/// 2. Using workspace_id to get the correct storage path
/// 3. Preventing path traversal attacks via pattern validation
///
/// Files from other workspaces cannot be accessed because ripgrep operates
/// within the workspace's isolated directory structure.
pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        r#"Finds files matching glob patterns. Requires ripgrep.

PATTERNS: *.rs, **/*.md, /src/**/*.rs
PARAMETERS: pattern (required), path (default '/')"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g., '*.rs', '**/*.md', '/src/**/*.rs')"
                },
                "path": {
                    "type": ["string", "null"],
                    "description": "Base directory for search (default: '/' for workspace root)"
                }
            },
            "required": ["pattern"],
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
        let glob_args: GlobArgs = serde_json::from_value(args)?;
        let pattern = glob_args.pattern.trim();

        if pattern.is_empty() {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "pattern".to_string(),
                message: "Pattern cannot be empty".to_string(),
            }));
        }

        // Security: Validate pattern doesn't attempt to escape workspace
        let normalized_pattern = pattern.strip_prefix('/').unwrap_or(pattern);
        if normalized_pattern.contains("..") {
            return Ok(ToolResponse {
                success: false,
                result: Value::Null,
                error: Some("Pattern cannot contain '..' (parent directory reference)".to_string()),
            });
        }

        // Get workspace directory for security isolation
        let workspace_path = storage.get_workspace_path(workspace_id);

        // Check if workspace directory exists
        if !workspace_path.exists() {
            return Ok(ToolResponse {
                success: true,
                result: serde_json::to_value(GlobResult {
                    pattern: pattern.to_string(),
                    base_path: "/".to_string(),
                    matches: Vec::new(),
                })?,
                error: None,
            });
        }

        // Normalize base path for ripgrep
        let base_path = super::normalize_path(&glob_args.path.unwrap_or_else(|| "/".to_string()));

        // Build ripgrep command for file discovery
        let mut cmd = build_glob_command(normalized_pattern, &base_path, &workspace_path)?;

        tracing::debug!("Executing glob command: rg --files --glob {}", normalized_pattern);

        // Execute command
        let output = cmd.output().await.map_err(|e| {
            Error::Internal(format!("Failed to execute ripgrep command: {}", e))
        })?;

        // Handle exit codes
        // 0 = matches found
        // 1 = no matches found (successful search, just no results)
        // >1 = error
        if !output.status.success() {
            let code = output.status.code().unwrap_or(2);
            if code == 1 {
                // No matches - return success with empty list
                tracing::debug!("Glob found no matches");
                return Ok(ToolResponse {
                    success: true,
                    result: serde_json::to_value(GlobResult {
                        pattern: pattern.to_string(),
                        base_path,
                        matches: Vec::new(),
                    })?,
                    error: None,
                });
            }

            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Glob command failed (code {}): {}", code, stderr);
            return Ok(ToolResponse {
                success: false,
                result: Value::Null,
                error: Some(format!("Glob command failed: {}", stderr)),
            });
        }

        // Parse output - each line is a file path
        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::debug!("Glob stdout length: {} bytes", stdout.len());

        let mut matches = Vec::new();

        for file_path in stdout.lines() {
            // Convert relative path from ripgrep to workspace path format
            let workspace_relative_path = file_path.strip_prefix("./").unwrap_or(file_path);
            let full_path = format!("/{}", workspace_relative_path);

            // Get metadata from database to enrich the result
            if let Ok(Some(file)) = files::get_file_by_path(conn, workspace_id, &full_path).await {
                matches.push(GlobMatch {
                    path: file.path.clone(),
                    name: file.name.clone(),
                    synced: true,  // Database entry
                    file_type: file.file_type,
                    is_virtual: file.is_virtual,
                    size: None, // Size would require additional storage access
                    updated_at: file.updated_at,
                });
            } else {
                // File exists on disk but not in database - add with minimal info
                // This can happen for files created externally
                let path_obj = Path::new(file_path);
                let name = path_obj
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                matches.push(GlobMatch {
                    path: full_path.clone(),
                    name,
                    synced: false,  // Filesystem-only
                    file_type: crate::models::files::FileType::Document, // Default to document
                    is_virtual: false,
                    size: None,
                    updated_at: chrono::Utc::now(),
                });
            }
        }

        tracing::debug!("Found {} matches", matches.len());

        // Sort matches by path for deterministic output
        matches.sort_by(|a, b| a.path.cmp(&b.path));

        let result = GlobResult {
            pattern: pattern.to_string(),
            base_path,
            matches,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

/// Builds the ripgrep command for glob file discovery
///
/// Requires ripgrep (rg) to be installed. Returns an error if rg is not available.
///
/// # Security
///
/// The command uses current_dir to restrict ripgrep to the workspace directory,
/// preventing access to files outside the workspace.
fn build_glob_command(
    pattern: &str,
    base_path: &str,
    workspace_path: &Path,
) -> Result<TokioCommand> {
    // Check if ripgrep is available
    if StdCommand::new("rg")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some()
    {
        tracing::debug!("Using ripgrep for glob");

        let mut cmd = TokioCommand::new("rg");

        // SECURITY: Set current directory to workspace path to isolate the search
        // This prevents ripgrep from accessing files outside this workspace
        cmd.current_dir(workspace_path);

        // Use --files to list files without searching content
        cmd.arg("--files");

        // Add glob pattern for filtering
        cmd.arg("--glob").arg(pattern);

        // Determine search directory based on base_path
        // If base_path is "/" (root), search from "."
        // Otherwise, search from the specified relative path
        let search_dir = if base_path == "/" {
            "."
        } else {
            // Strip leading slash to get relative path from workspace root
            base_path.strip_prefix('/').unwrap_or(base_path)
        };

        cmd.arg(search_dir);

        return Ok(cmd);
    }

    Err(Error::Internal(
        "ripgrep (rg) not found on system. Required for glob tool.".to_string()
    ))
}
