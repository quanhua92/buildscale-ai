use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, FindArgs, FindResult, FindMatch}, queries::files};
use crate::services::storage::FileStorageService;
use crate::models::files::FileType;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use std::process::Command as StdCommand;
use tokio::process::Command as TokioCommand;
use std::path::Path;
use super::{Tool, ToolConfig};

/// Find tool for searching files by metadata using Unix find command
///
/// Finds files matching metadata criteria using the Unix `find` command for
/// filesystem discovery, then enriches results with database metadata.
///
/// # Security: Workspace Isolation
///
/// This tool ensures workspace isolation by:
/// 1. Setting find's current directory to the workspace root
/// 2. Using workspace_id to get the correct storage path
/// 3. Validating path patterns to prevent traversal attacks
///
/// Files from other workspaces cannot be accessed because find operates
/// within the workspace's isolated directory structure.
pub struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &'static str {
        "find"
    }

    fn description(&self) -> &'static str {
        r#"Finds files by metadata (name, path, type, size, date). Uses Unix find command for filesystem discovery, then enriches with database metadata. Complements grep which searches by content.

SEARCH PARAMETERS:
- name: Filename pattern (supports find wildcards: *, ?, [])
- path: Directory to search (default: workspace root)
- file_type: Filter by type (document, folder, canvas)
- min_size: Minimum file size (e.g., 1048576 for 1MB)
- max_size: Maximum file size
- recursive: Search subdirectories (default: true)

DIFFERENCES FROM OTHER TOOLS:
- find: Searches by metadata (name, type, size, date)
- grep: Searches by content (text within files)
- glob: Pattern matching for filenames only
- ls: Lists directory contents

USE CASES:
- Find all files larger than 1MB
- Find all folders in a directory
- Find all canvas files
- Find files modified recently

EXAMPLES:
{"name": "*.txt", "recursive": true} - All text files
{"path": "/src", "name": "*.rs"} - Rust files under src
{"file_type": "folder"} - All folders
{"min_size": 1048576} - Files larger than 1MB

REQUIREMENTS: Requires Unix find command to be installed on the system."#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": ["string", "null"],
                    "description": "Filename pattern with wildcards (e.g., '*.txt', 'test_*')"
                },
                "path": {
                    "type": ["string", "null"],
                    "description": "Base directory for search (default: '/' for workspace root)"
                },
                "file_type": {
                    "type": ["string", "null"],
                    "description": "File type filter (document, folder, canvas, etc.)"
                },
                "min_size": {
                    "type": ["integer", "string", "null"],
                    "description": "Minimum file size in bytes. Accepts integer or string (e.g., 1048576 or '1048576')."
                },
                "max_size": {
                    "type": ["integer", "string", "null"],
                    "description": "Maximum file size in bytes. Accepts integer or string (e.g., 10485760 or '10485760')."
                },
                "recursive": {
                    "type": ["boolean", "string", "null"],
                    "description": "Search subdirectories (default: true). Accepts boolean or string (e.g., true or 'true')."
                }
            },
            "required": [],
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
        let args: FindArgs = serde_json::from_value(args)?;

        // Normalize path pattern if provided
        let base_path = if let Some(ref path_pattern) = args.path {
            super::normalize_path(path_pattern)
        } else {
            "/".to_string()
        };

        // Security: Validate path doesn't attempt to escape workspace
        if base_path.contains("..") {
            return Ok(ToolResponse {
                success: false,
                result: Value::Null,
                error: Some("Path cannot contain '..' (parent directory reference)".to_string()),
            });
        }

        // Get workspace directory for security isolation
        let workspace_path = storage.get_workspace_path(workspace_id);

        // Check if workspace directory exists
        if !workspace_path.exists() {
            return Ok(ToolResponse {
                success: true,
                result: serde_json::to_value(FindResult {
                    matches: Vec::new(),
                })?,
                error: None,
            });
        }

        // Verify base_path exists and is a directory (if not root)
        if base_path != "/" {
            let parent_file = files::get_file_by_path(conn, workspace_id, &base_path).await?;
            if let Some(file) = parent_file {
                if !matches!(file.file_type, FileType::Folder) {
                    return Err(Error::Validation(crate::error::ValidationErrors::Single {
                        field: "path".to_string(),
                        message: format!("Base path is not a directory: {}", base_path),
                    }));
                }
            } else {
                return Err(Error::NotFound(format!("Base path not found: {}", base_path)));
            }
        }

        // Determine if recursive (default: true)
        let recursive = args.recursive.unwrap_or(true);

        // Build find command
        let mut cmd = build_find_command(
            args.name.as_deref(),
            &base_path,
            recursive,
            args.min_size,
            args.max_size,
            args.file_type.as_ref(),
            &workspace_path,
        )?;

        tracing::debug!("Executing find command in directory: {:?}", workspace_path);

        // Execute command
        let output = cmd.output().await.map_err(|e| {
            Error::Internal(format!("Failed to execute find command: {}", e))
        })?;

        // Handle exit codes
        // 0 = matches found
        // 1 = no matches found (successful search)
        // >1 = error
        if !output.status.success() {
            let code = output.status.code().unwrap_or(2);
            if code == 1 {
                // No matches - return success with empty list
                tracing::debug!("Find found no matches");
                return Ok(ToolResponse {
                    success: true,
                    result: serde_json::to_value(FindResult {
                        matches: Vec::new(),
                    })?,
                    error: None,
                });
            }

            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Find command failed (code {}): {}", code, stderr);
            return Ok(ToolResponse {
                success: false,
                result: Value::Null,
                error: Some(format!("Find command failed: {}", stderr)),
            });
        }

        // Parse output - each line is a file path
        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::debug!("Find stdout length: {} bytes", stdout.len());

        let mut matches = Vec::new();

        for file_path in stdout.lines() {
            // Skip "." (current directory) which find always returns
            if file_path == "." || file_path == "./." {
                continue;
            }

            // Convert relative path from find to workspace path format
            let workspace_relative_path = file_path.strip_prefix("./").unwrap_or(file_path);
            let full_path = format!("/{}", workspace_relative_path);

            // Get file size using stat command (portable: works on both Linux and macOS)
            let size = if let Ok(metadata) = tokio::fs::metadata(workspace_path.join(&workspace_relative_path)).await {
                Some(metadata.len() as usize)
            } else {
                None
            };

            // Get metadata from database to enrich the result
            if let Ok(Some(file)) = files::get_file_by_path(conn, workspace_id, &full_path).await {
                // Filter by file_type if specified
                // Convert FileType enum to string for comparison with target_type
                if let Some(ref target_type) = args.file_type {
                    if file.file_type.to_string() != target_type.to_string() {
                        continue;
                    }
                }

                matches.push(FindMatch {
                    path: file.path.clone(),
                    name: file.name.clone(),
                    file_type: file.file_type,
                    size, // Use actual file size from filesystem stat
                    updated_at: file.updated_at,
                });
            }
        }

        tracing::debug!("Found {} matches after database filtering", matches.len());

        // Sort matches by path for deterministic output
        matches.sort_by(|a, b| a.path.cmp(&b.path));

        let result = FindResult {
            matches,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

/// Builds the Unix find command for file discovery
///
/// Requires the find command to be installed. Returns an error if find is not available.
///
/// # Security
///
/// The command uses current_dir to restrict find to the workspace directory,
/// preventing access to files outside the workspace.
///
/// # Size Format
///
/// Uses find's size format:
/// - `b` for 512-byte blocks (default)
/// - `c` for bytes
/// - `k` for kilobytes
/// - `M` for megabytes
/// - `G` for gigabytes
///
/// Examples:
/// - `1048576c` = exactly 1,048,576 bytes
/// - `+1048576c` = greater than 1MB
/// - `-10485760c` = less than 10MB
///
/// # Size Retrieval Strategy
///
/// Uses stat command to get file sizes (portable across GNU and BSD find).
/// This is necessary because -printf is GNU-find-only and not available on macOS.
fn build_find_command(
    name_pattern: Option<&str>,
    base_path: &str,
    recursive: bool,
    min_size: Option<usize>,
    max_size: Option<usize>,
    file_type: Option<&FileType>,
    workspace_path: &Path,
) -> Result<TokioCommand> {
    // Check if find is available
    // Note: Both GNU and BSD find support basic operations, so we test with a simple command
    if StdCommand::new("find")
        .arg(".")
        .arg("-maxdepth")
        .arg("0")
        .output()
        .ok()
        .is_some()
    {
        tracing::debug!("Using find for metadata search");

        let mut cmd = TokioCommand::new("find");

        // SECURITY: Set current directory to workspace path to isolate the search
        // This prevents find from accessing files outside this workspace
        cmd.current_dir(workspace_path);

        // Add search directory (relative to workspace root)
        let search_dir = if base_path == "/" {
            "."
        } else {
            // Strip leading slash to get relative path from workspace root
            base_path.strip_prefix('/').unwrap_or(base_path)
        };
        cmd.arg(search_dir);

        // Add recursive/non-recursive flag
        if !recursive {
            cmd.arg("-maxdepth").arg("1");
        }

        // Add type filter if file_type is specified
        // Map our FileType to find's type options:
        // - folder -> d (directory)
        // - document -> f (regular file)
        // - canvas, other -> f (regular file)
        if let Some(ft) = file_type {
            let find_type = match ft {
                FileType::Folder => "d",
                _ => "f", // All non-folder types are regular files
            };
            cmd.arg("-type").arg(find_type);
        }

        // Add name pattern if provided
        if let Some(name) = name_pattern {
            cmd.arg("-name").arg(name);
        }

        // Add size filters if provided
        // find size format: n[cwbkMG] (c=bytes, k=KB, M=MB, G=GB)
        if let Some(min) = min_size {
            cmd.arg("-size").arg(format!("+{}c", min));
        }
        if let Some(max) = max_size {
            cmd.arg("-size").arg(format!("-{}c", max));
        }

        // Print file paths (will use stat to get sizes - portable across GNU/BSD find)
        cmd.arg("-print");

        return Ok(cmd);
    }

    Err(Error::Internal(
        "find command not found on system. Required for find tool.".to_string()
    ))
}
