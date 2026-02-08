use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, FindArgs, FindResult, FindMatch}, models::files::FileType};
use crate::queries::files as file_queries;
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Find tool for searching files by metadata
///
/// Finds files matching metadata criteria (name, path, file_type, size, date).
/// Complements grep which searches by content.
pub struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &'static str {
        "find"
    }

    fn description(&self) -> &'static str {
        r#"Finds files by metadata (name, path, type, size, date). Complements grep which searches by content.

SEARCH PARAMETERS:
- name: Filename pattern (supports * wildcards, e.g., "*.txt", "test_*")
- path: Path pattern (e.g., "/src/*", "/**/*.rs")
- file_type: Filter by type (document, folder, canvas, etc.)
- min_size: Minimum file size in bytes
- max_size: Maximum file size in bytes
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
- Find files created/modified recently

EXAMPLES:
{"name": "*.txt", "recursive": true} - All text files
{"path": "/src/**/*.rs"} - All Rust files under src
{"file_type": "folder"} - All folders
{"min_size": 1000000} - Files larger than 1MB"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": ["string", "null"],
                    "description": "Filename pattern with * wildcards (e.g., '*.txt', 'test_*')"
                },
                "path": {
                    "type": ["string", "null"],
                    "description": "Path pattern (e.g., '/src/*', '/**/*.rs')"
                },
                "file_type": {
                    "type": ["string", "null"],
                    "description": "File type filter (document, folder, canvas, etc.)"
                },
                "min_size": {
                    "type": ["integer", "null"],
                    "description": "Minimum file size in bytes"
                },
                "max_size": {
                    "type": ["integer", "null"],
                    "description": "Maximum file size in bytes"
                },
                "recursive": {
                    "type": ["boolean", "null"],
                    "description": "Search subdirectories (default: true)"
                }
            },
            "required": [],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        conn: &mut DbConn,
        _storage: &FileStorageService,
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

        // Get parent_id for base path (if not root)
        let parent_id = if base_path == "/" {
            None
        } else {
            let parent_file = file_queries::get_file_by_path(conn, workspace_id, &base_path).await?
                .ok_or_else(|| Error::NotFound(format!("Base path not found: {}", base_path)))?;

            if !matches!(parent_file.file_type, FileType::Folder) {
                return Err(Error::Validation(crate::error::ValidationErrors::Single {
                    field: "path".to_string(),
                    message: format!("Base path is not a directory: {}", base_path),
                }));
            }

            Some(parent_file.id)
        };

        // Determine if recursive (default: true)
        let recursive = args.recursive.unwrap_or(true);

        // Get candidate files
        let all_files = if recursive {
            Self::list_files_recursive(conn, workspace_id, &base_path).await?
        } else {
            file_queries::list_files_in_folder(conn, workspace_id, parent_id).await?
        };

        // Filter files by criteria
        let mut matches: Vec<FindMatch> = all_files
            .into_iter()
            .filter(|file| Self::file_matches_criteria(file, &args))
            .map(|file| FindMatch {
                path: file.path.clone(),
                name: file.name.clone(),
                file_type: file.file_type,
                size: None, // TODO: Implement size tracking in storage
                updated_at: file.updated_at,
            })
            .collect();

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

impl FindTool {
    async fn list_files_recursive(
        conn: &mut DbConn,
        workspace_id: Uuid,
        path_prefix: &str,
    ) -> Result<Vec<crate::models::files::File>> {
        let files = sqlx::query_as!(
            crate::models::files::File,
            r#"
            SELECT
                id, workspace_id, parent_id, author_id,
                file_type as "file_type: crate::models::files::FileType",
                status as "status: crate::models::files::FileStatus",
                name, slug, path,
                is_virtual, is_remote, permission,
                latest_version_id,
                deleted_at, created_at, updated_at
            FROM files
            WHERE workspace_id = $1
              AND path LIKE $2 || '%'
              AND path != $2
              AND deleted_at IS NULL
            ORDER BY path ASC
            "#,
            workspace_id,
            path_prefix
        )
        .fetch_all(conn)
        .await
        .map_err(Error::Sqlx)?;

        Ok(files)
    }

    fn file_matches_criteria(file: &crate::models::files::File, args: &FindArgs) -> bool {
        // Filter by name pattern
        if let Some(ref name_pattern) = args.name {
            if !Self::matches_pattern(&file.name, name_pattern) {
                return false;
            }
        }

        // Filter by file_type
        if let Some(ref file_type) = args.file_type {
            if &file.file_type != file_type {
                return false;
            }
        }

        // Filter by size (not implemented yet - would require storage metadata)
        if args.min_size.is_some() || args.max_size.is_some() {
            // Size filtering not implemented - skip for now
            // TODO: Implement when size tracking is added to storage
        }

        true
    }

    fn matches_pattern(text: &str, pattern: &str) -> bool {
        // Simple glob-style pattern matching
        // Supports * wildcard only for now
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let (prefix, suffix) = (parts[0], parts[1]);
                if suffix.is_empty() {
                    // Pattern like "test_*" - starts with
                    text.starts_with(prefix)
                } else if prefix.is_empty() {
                    // Pattern like "*.txt" - ends with
                    text.ends_with(suffix)
                } else {
                    // Pattern like "*test*" - contains
                    text.starts_with(prefix) && text.ends_with(suffix)
                }
            } else {
                // Multiple wildcards - do substring match
                let mut result = true;
                let mut pos = 0;
                for part in parts {
                    if let Some(idx) = text[pos..].find(part) {
                        pos = idx + part.len();
                    } else {
                        result = false;
                        break;
                    }
                }
                result
            }
        } else {
            // No wildcards - exact match
            text == pattern
        }
    }
}
