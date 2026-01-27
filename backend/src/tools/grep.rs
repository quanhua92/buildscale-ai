use crate::{DbConn, error::Result};
use crate::models::files::FileType;
use crate::models::requests::{ToolResponse, GrepArgs, GrepMatch, GrepResult};
use crate::queries::files as file_queries;
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::Tool;

/// Grep tool for searching file contents using regex
///
/// Searches for a regex pattern across all document files in a workspace.
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Searches for a regex pattern across all document files in a workspace using disk-based operations. Pattern is required. Optional path_pattern filters results (supports * wildcards). Set case_sensitive to false (default) for case-insensitive search. Returns matching file paths with context. Use for discovering code patterns or finding specific content."
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

        // Validate regex pattern
        let regex = match grep_args.case_sensitive.unwrap_or(false) {
            true => regex::Regex::new(&grep_args.pattern),
            false => regex::RegexBuilder::new(&grep_args.pattern).case_insensitive(true).build(),
        };

        let regex = match regex {
            Ok(re) => re,
            Err(e) => {
                return Ok(ToolResponse {
                    success: false,
                    result: serde_json::Value::Null,
                    error: Some(format!("Invalid regex pattern: {}", e)),
                });
            }
        };

        // Convert path_pattern to glob pattern if provided
        let path_glob = grep_args.path_pattern.as_ref().map(|p| {
            let mut glob_pattern = p.clone();
            // Convert * to glob pattern
            glob_pattern = glob_pattern.replace('*', "**");
            glob_pattern
        });

        // Get all active files in workspace
        let all_files = file_queries::list_all_active_files(conn, workspace_id).await?;

        let mut matches = Vec::new();
        let mut match_count = 0;
        const MAX_MATCHES: usize = 1000;

        for file in all_files {
            if match_count >= MAX_MATCHES {
                break;
            }

            // Skip folders
            if matches!(file.file_type, FileType::Folder) {
                continue;
            }

            // Apply path filter if provided
            if let Some(ref glob_pattern) = path_glob {
                if !matches_path_pattern(&file.path, glob_pattern) {
                    continue;
                }
            }

            // Read file content from disk
            let file_bytes = match storage.read_file(workspace_id, &file.path).await {
                Ok(content) => content,
                Err(e) => {
                    tracing::warn!("Failed to read file {} for grep: {}", file.path, e);
                    continue;
                }
            };

            // Convert bytes to string, defaulting to empty if not valid UTF-8
            let file_content = String::from_utf8_lossy(&file_bytes);

            // Search for pattern in file content
            for (line_num, line) in file_content.lines().enumerate() {
                if regex.is_match(line) {
                    matches.push(GrepMatch {
                        path: file.path.clone(),
                        line_number: (line_num + 1) as i32,
                        line_text: line.to_string(),
                    });
                    match_count += 1;
                    if match_count >= MAX_MATCHES {
                        break;
                    }
                }
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

/// Matches a file path against a glob pattern
fn matches_path_pattern(path: &str, pattern: &str) -> bool {
    let pattern = pattern.trim_start_matches('/');

    if pattern.is_empty() {
        return true;
    }

    if pattern.contains('*') {
        // Simple glob matching
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut path_idx = 0;

        for part in &parts {
            if part.is_empty() {
                continue;
            }

            if let Some(idx) = path[path_idx..].find(part) {
                path_idx += idx + part.len();
            } else {
                return false;
            }
        }

        // If pattern ends with *, anything after last match is okay
        if pattern.ends_with('*') {
            return true;
        }

        // Otherwise, we must have consumed the entire path
        path_idx >= path.len()
    } else {
        // Exact match or prefix match
        path.starts_with(pattern)
    }
}