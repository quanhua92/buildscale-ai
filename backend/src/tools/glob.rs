use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, GlobArgs, GlobResult, GlobMatch}, queries::files};
use crate::models::files::File;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Pattern matching helper for glob patterns
///
/// Checks if a file path matches a glob pattern
///
/// This is used for additional filtering after SQL queries
/// since SQL LIKE may not cover all glob edge cases.
///
/// NOTE: We use a custom implementation instead of glob/globset crates because:
/// 1. SQL queries already do most of the filtering work (path prefix matching)
/// 2. We only need to handle common simple patterns (*.rs, **/*.md, etc.)
/// 3. The database-backed file system doesn't map directly to filesystem paths
/// 4. Custom logic allows tight integration with our SQL-based pattern matching
/// 5. The patterns we support are intentionally limited to well-defined cases
fn path_matches_glob(file_path: &str, pattern: &str) -> bool {
    let file_path = file_path.strip_prefix('/').unwrap_or(file_path);
    let pattern = pattern.strip_prefix('/').unwrap_or(pattern);

    // Handle common glob patterns
    if pattern.contains("**") {
        // ** matches anything including slashes
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0].trim_end_matches('/'), parts[1].trim_start_matches('/'));
            if suffix.is_empty() {
                // Pattern like "src/**" - matches everything under src
                return file_path.starts_with(prefix) || file_path == prefix;
            }
            // Pattern like "**/*.rs" - matches any .rs file
            if prefix.is_empty() {
                // Suffix might be "*.rs" where * is a single wildcard, not globstar
                // We need to match paths ending with .rs (and not containing / in the suffix part after **)
                // For "**/*.rs", suffix is "/*.rs", we need to match paths where the last segment matches *.rs
                // Extract just the filename pattern from suffix (after any slashes)
                let filename_pattern = suffix.trim_start_matches('/');
                if filename_pattern.contains('*') {
                    // Simple case: *.ext pattern - check if file ends with .ext
                    // Replace * with empty string to get just .ext
                    let ext = filename_pattern.replace('*', "");
                    return file_path.ends_with(&ext) && !file_path[file_path.len() - ext.len()..].contains('/');
                }
                return file_path.ends_with(suffix);
            }
            // Pattern like "src/**/*.rs"
            // File must start with prefix and end with a path matching the suffix pattern
            if !file_path.starts_with(prefix) {
                return false;
            }
            // For "src/**/*.rs", suffix is "/*.rs"
            // We need to check if the path after prefix matches *.rs
            let remaining = &file_path[prefix.len()..];
            let filename_pattern = suffix.trim_start_matches('/');
            if filename_pattern.contains('*') {
                // Simple case: *.ext pattern
                let ext = filename_pattern.replace('*', "");
                return remaining.ends_with(&ext) && !remaining[remaining.len() - ext.len()..].contains('/');
            }
            return remaining.ends_with(suffix);
        }
    }

    if pattern.contains('*') {
        // Split by * and check if path matches
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0].trim_end_matches('/'), parts[1]);
            if !suffix.is_empty() {
                // Check suffix match (e.g., .rs extension)
                file_path.starts_with(prefix)
                    && file_path.ends_with(suffix)
            } else {
                // Pattern like "src/*"
                file_path.starts_with(prefix)
            }
        } else {
            // Multiple wildcards, do substring match
            file_path.contains(&pattern.replace('*', ""))
        }
    } else {
        // No wildcards - exact match or directory prefix
        file_path == pattern
            || file_path.starts_with(&format!("{}/", pattern))
            || file_path == pattern.trim_end_matches('/')
    }
}

/// Glob tool for pattern-based file search
///
/// Finds files matching glob patterns like `*.rs`, `**/*.md`, `/src/**/*.rs`.
pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        r#"Finds files matching glob patterns. Faster than recursive ls for pattern searches. Returns matches with metadata (path, file_type, size, updated_at).

SUPPORTED PATTERNS:
- *.rs - matches all .rs files anywhere in workspace
- **/*.md - matches all .md files recursively
- /src/**/*.rs - matches all .rs files under src/
- test_* - matches files/folders starting with test_
- */file.txt - matches file.txt in any immediate subdirectory

DIFFERENCES FROM LS:
- glob: Pattern matching (e.g., '*.rs' finds all Rust files)
- ls: Directory listing (e.g., '/src' lists contents of /src folder)

USE GLOB WHEN:
- Searching files by extension (*.rs, *.md)
- Finding files matching naming patterns (test_*, config.*)
- Quick filtering without reading file contents

USE LS WHEN:
- Browsing directory contents
- Need recursive listing of all files
- Exploring folder structure

PERFORMANCE: Glob is optimized for pattern matching and reduces token usage vs ls + manual filtering."#
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
        _storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let glob_args: GlobArgs = serde_json::from_value(args)?;
        let pattern = glob_args.pattern.trim();
        let base_path = super::normalize_path(&glob_args.path.unwrap_or_else(|| "/".to_string()));

        if pattern.is_empty() {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "pattern".to_string(),
                message: "Pattern cannot be empty".to_string(),
            }));
        }

        // Get parent_id for base path if it's not root
        let parent_id = if base_path == "/" {
            None
        } else {
            let parent_file = files::get_file_by_path(conn, workspace_id, &base_path).await?
                .ok_or_else(|| Error::NotFound(format!("Base path not found: {}", base_path)))?;

            if !matches!(parent_file.file_type, crate::models::files::FileType::Folder) {
                return Err(Error::Validation(crate::error::ValidationErrors::Single {
                    field: "path".to_string(),
                    message: format!("Base path is not a directory: {}", base_path),
                }));
            }

            Some(parent_file.id)
        };

        // Determine if pattern is recursive (contains ** or starts with *)
        let is_recursive = pattern.contains("**") || pattern.starts_with('*');

        // Get candidate files from database
        let all_files = if is_recursive {
            // For recursive patterns, get all files under base path
            Self::list_files_recursive(conn, workspace_id, &base_path).await?
        } else {
            // For non-recursive patterns, get direct children only
            files::list_files_in_folder(conn, workspace_id, parent_id).await?
        };

        // Filter files by glob pattern
        let mut matches: Vec<GlobMatch> = all_files
            .into_iter()
            .filter(|file| path_matches_glob(&file.path, pattern))
            .map(|file| GlobMatch {
                path: file.path.clone(),
                name: file.name.clone(),
                file_type: file.file_type,
                is_virtual: file.is_virtual,
                size: None, // Size would require storage access
                updated_at: file.updated_at,
            })
            .collect();

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

impl GlobTool {
    async fn list_files_recursive(
        conn: &mut DbConn,
        workspace_id: Uuid,
        path_prefix: &str,
    ) -> Result<Vec<File>> {
        let files = sqlx::query_as!(
            File,
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
}
