//! Memory search tool - searches memory files with scope, category, and tag filtering.
//!
//! Uses pure grep for efficient pattern matching, then filters results based on memory metadata.
//! Returns one match per unique memory file with a content preview.

use crate::error::{Error, Result};
use crate::models::requests::{
    ToolResponse, MemorySearchArgs, MemorySearchResult, MemoryMatch,
};
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{parse_memory_frontmatter, parse_memory_path, MemoryScope};
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use tokio::process::Command as TokioCommand;
use uuid::Uuid;

/// Default maximum number of search results
const DEFAULT_SEARCH_LIMIT: usize = 50;

/// Maximum words to include in content preview
const CONTENT_PREVIEW_WORDS: usize = 100;

/// Bytes to read from file head for frontmatter and preview (8KB)
const FILE_HEAD_BUFFER_SIZE: usize = 8192;

pub struct MemorySearchTool;

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &'static str {
        "memory_search"
    }

    fn description(&self) -> &'static str {
        r#"Searches stored memories by pattern with optional filtering.

Supports filtering by scope, category, and tags. Returns unique memory files with content preview.

Examples:
- Search all memories: {"pattern": "API key"}
- Search user memories: {"pattern": "preference", "scope": "user"}
- Filter by category: {"pattern": "config", "category": "project"}
- Filter by tags: {"pattern": "typescript", "tags": ["coding", "frontend"]}"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Search pattern (regex supported)"
                },
                "scope": {
                    "type": ["string", "null"],
                    "enum": ["user", "global", null],
                    "description": "Filter by scope: 'user' or 'global'"
                },
                "category": {
                    "type": ["string", "null"],
                    "description": "Filter by category"
                },
                "tags": {
                    "type": ["array", "null"],
                    "items": {"type": "string"},
                    "description": "Filter by tags (memory must have ALL specified tags)"
                },
                "case_sensitive": {
                    "type": ["boolean", "string", "null"],
                    "description": "Case-sensitive search (default: false)"
                },
                "limit": {
                    "type": ["integer", "string", "null"],
                    "description": "Maximum matches to return (default: 50, 0 for unlimited)"
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
        user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let search_args: MemorySearchArgs = serde_json::from_value(args)?;

        let limit = search_args.limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
        let case_sensitive = search_args.case_sensitive.unwrap_or(false);

        let workspace_path = storage.get_workspace_path(workspace_id);

        // Build grep patterns based on scope and category filters
        let grep_patterns = build_grep_patterns(
            &search_args.scope,
            search_args.category.as_deref(),
            user_id,
        );

        // Use grep to find files matching the pattern (returns file paths only)
        let matching_files = run_grep_for_memories(
            &search_args.pattern,
            &grep_patterns,
            case_sensitive,
            &workspace_path,
        ).await?;

        // If no matches from grep, return early
        if matching_files.is_empty() {
            return Ok(ToolResponse {
                success: true,
                result: serde_json::to_value(MemorySearchResult {
                    matches: Vec::new(),
                    total: 0,
                })?,
                error: None,
            });
        }

        let mut all_matches: Vec<MemoryMatch> = Vec::new();
        let mut seen_files: HashSet<String> = HashSet::new();

        // Process each file that matched grep (deduplicated)
        for file_path in &matching_files {
            // Skip if already processed (shouldn't happen but safety check)
            if !seen_files.insert(file_path.clone()) {
                continue;
            }

            // Parse scope, category, key from path
            let (scope, category, key) = match parse_memory_path(file_path) {
                Some(result) => result,
                None => continue,
            };

            // Apply scope filter
            if let Some(ref filter_scope) = search_args.scope {
                if &scope != filter_scope {
                    continue;
                }
            }

            // For user-scoped memories, verify ownership
            if scope == MemoryScope::User {
                let expected_prefix = format!("/users/{}/memories/", user_id);
                if !file_path.starts_with(&expected_prefix) {
                    continue; // Skip other users' memories
                }
            }

            // Apply category filter
            if let Some(ref filter_category) = search_args.category {
                if &category != filter_category {
                    continue;
                }
            }

            // Read file head for frontmatter and body preview (efficient, only first 8KB)
            let full_path = workspace_path.join(file_path.trim_start_matches('/'));
            let content = match read_file_head(&full_path).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(path = %file_path, error = %e, "Failed to read memory file");
                    continue;
                }
            };

            // Parse frontmatter to get metadata and body content
            let (metadata, body_content) = parse_memory_frontmatter(&content);

            // Apply tags filter
            if let Some(ref filter_tags) = search_args.tags {
                if let Some(ref mem_metadata) = metadata {
                    let has_all_tags = filter_tags.iter().all(|tag| {
                        mem_metadata.tags.contains(tag)
                    });
                    if !has_all_tags {
                        continue;
                    }
                } else {
                    continue; // No metadata, can't verify tags
                }
            }

            // Get file updated_at from filesystem
            let updated_at = match tokio::fs::metadata(&full_path).await {
                Ok(meta) => {
                    meta.modified()
                        .ok()
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or_else(chrono::Utc::now)
                }
                Err(_) => chrono::Utc::now(),
            };

            // Build metadata for match
            let mem_metadata = metadata.clone().unwrap_or_else(|| crate::utils::MemoryMetadata {
                title: key.clone(),
                tags: vec![],
                category: category.clone(),
                created_at: chrono::Utc::now(),
                updated_at,
                scope: scope.clone(),
            });

            // Extract content preview from body
            let content_preview = extract_content_preview(&body_content, CONTENT_PREVIEW_WORDS);

            // Create single match per file
            all_matches.push(MemoryMatch {
                path: file_path.clone(),
                scope,
                category,
                key,
                title: mem_metadata.title,
                content_preview,
                tags: mem_metadata.tags,
                updated_at,
            });
        }

        // Sort by updated_at descending (most recent first)
        all_matches.sort_by(|a, b| {
            b.updated_at.cmp(&a.updated_at)
        });

        // Apply limit after sorting to get most recent matches
        let total = all_matches.len();
        if limit > 0 && all_matches.len() > limit {
            all_matches.truncate(limit);
        }

        let result = MemorySearchResult {
            total,
            matches: all_matches,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

/// Build grep glob patterns based on scope and category filters
fn build_grep_patterns(
    scope_filter: &Option<MemoryScope>,
    category_filter: Option<&str>,
    user_id: Uuid,
) -> Vec<String> {
    // Build the path suffix based on category filter
    // If category is specified, search only that category directory
    // Otherwise, search all categories with recursive glob
    let category_suffix = match category_filter {
        Some(cat) => format!("{}/*.md", cat),  // Specific category, non-recursive
        None => "**/*.md".to_string(),          // All categories, recursive
    };

    match scope_filter {
        Some(MemoryScope::User) => {
            // Only search user's memory directory
            vec![format!("users/{}/memories/{}", user_id, category_suffix)]
        }
        Some(MemoryScope::Global) => {
            // Only search global memory directory
            vec![format!("memories/{}", category_suffix)]
        }
        None => {
            // Search both user and global directories
            vec![
                format!("memories/{}", category_suffix),
                format!("users/{}/memories/{}", user_id, category_suffix),
            ]
        }
    }
}

/// Run grep on memory directories and return deduplicated file paths
async fn run_grep_for_memories(
    pattern: &str,
    glob_patterns: &[String],
    case_sensitive: bool,
    workspace_path: &Path,
) -> Result<Vec<String>> {
    let mut all_files: HashSet<String> = HashSet::new();

    for glob_pattern in glob_patterns {
        // Try ripgrep first, then fall back to standard grep
        let files = run_ripgrep_glob(pattern, glob_pattern, case_sensitive, workspace_path).await;
        let files = match files {
            Ok(f) => f,
            Err(_) => run_standard_grep_glob(pattern, glob_pattern, case_sensitive, workspace_path).await?,
        };

        // Merge file paths into result
        all_files.extend(files);
    }

    // Convert to sorted vector for consistent ordering
    let mut result: Vec<String> = all_files.into_iter().collect();
    result.sort();
    Ok(result)
}

/// Read only the beginning of a file for frontmatter and preview (more efficient than reading entire file)
async fn read_file_head(path: &Path) -> Result<String> {
    use tokio::io::{AsyncReadExt, BufReader};

    let file = tokio::fs::File::open(path).await
        .map_err(|e| Error::Internal(format!("Failed to open file: {}", e)))?;

    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; FILE_HEAD_BUFFER_SIZE];

    let bytes_read = reader.read(&mut buffer).await
        .map_err(|e| Error::Internal(format!("Failed to read file: {}", e)))?;

    buffer.truncate(bytes_read);

    String::from_utf8(buffer)
        .map_err(|e| Error::Internal(format!("Invalid UTF-8 in file: {}", e)))
}

/// Extract content preview from body text (first N words)
fn extract_content_preview(body: &str, max_words: usize) -> String {
    let words: Vec<&str> = body.split_whitespace().take(max_words).collect();
    let total_words = body.split_whitespace().count();

    if total_words > max_words {
        format!("{}...", words.join(" "))
    } else {
        words.join(" ")
    }
}

/// Run ripgrep with glob pattern
async fn run_ripgrep_glob(
    pattern: &str,
    glob_pattern: &str,
    case_sensitive: bool,
    workspace_path: &Path,
) -> Result<Vec<String>> {
    // Extract directory from glob pattern for the search path
    // e.g., "users/123/memories/**/*.md" -> search in "users/123/memories"
    // e.g., "memories/**/*.md" -> search in "memories"
    let search_dir = glob_pattern
        .split('/')
        .take_while(|p| !p.contains('*'))
        .collect::<Vec<_>>()
        .join("/");

    let search_path = workspace_path.join(&search_dir);

    // Skip if directory doesn't exist
    if !search_path.exists() {
        tracing::debug!("Search directory does not exist: {:?}", search_path);
        return Ok(Vec::new());
    }

    let mut cmd = TokioCommand::new("rg");
    cmd.current_dir(&search_path);
    cmd.arg("--files-with-matches");  // Only return file paths, not line content
    cmd.arg("--glob");
    cmd.arg("**/*.md");  // Recursive glob to match files in all subdirectories

    if case_sensitive {
        cmd.arg("--case-sensitive");
    } else {
        cmd.arg("--ignore-case");
    }

    cmd.arg("--");
    cmd.arg(pattern);
    cmd.arg(".");

    tracing::debug!("Running ripgrep with pattern: {}, search_dir: {}, search_path: {:?}", pattern, search_dir, search_path);

    let output = cmd.output().await.map_err(|e| {
        tracing::error!("Failed to execute ripgrep: {}", e);
        Error::Internal(format!("Failed to execute ripgrep: {}", e))
    })?;

    tracing::debug!("ripgrep exit code: {:?}", output.status.code());

    // Exit code 1 means no matches, which is fine
    if !output.status.success() && output.status.code() != Some(1) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!("ripgrep returned non-zero: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ripgrep_files_output(&stdout, &search_dir)
}

/// Parse ripgrep --files-with-matches output into workspace-relative paths
fn parse_ripgrep_files_output(
    output: &str,
    search_dir: &str,
) -> Result<Vec<String>> {
    let mut files: Vec<String> = Vec::new();

    for line in output.lines() {
        let path = line.trim();
        if path.is_empty() {
            continue;
        }

        // Convert relative path to workspace-relative path
        let workspace_relative = if search_dir.is_empty() {
            format!("/{}", path.trim_start_matches("./"))
        } else {
            format!("/{}/{}", search_dir.trim_end_matches('/'), path.trim_start_matches("./"))
        };

        files.push(workspace_relative);
    }

    Ok(files)
}

/// Run standard grep with glob pattern (directory-based)
async fn run_standard_grep_glob(
    pattern: &str,
    glob_pattern: &str,
    case_sensitive: bool,
    workspace_path: &Path,
) -> Result<Vec<String>> {
    // Extract directory from glob pattern
    // e.g., "users/123/memories/**/*.md" -> "users/123/memories"
    let search_dir = glob_pattern
        .split('/')
        .take_while(|p| !p.contains('*'))
        .collect::<Vec<_>>()
        .join("/");

    let search_path = workspace_path.join(&search_dir);

    // Skip if directory doesn't exist
    if !search_path.exists() {
        return Ok(Vec::new());
    }

    let mut cmd = TokioCommand::new("grep");
    cmd.current_dir(&search_path);
    cmd.arg("-R");
    cmd.arg("-l");  // Only return file paths, not line content
    cmd.arg("--include=*.md");

    if !case_sensitive {
        cmd.arg("-i");
    }

    cmd.arg("--");
    cmd.arg(pattern);
    cmd.arg(".");

    let output = cmd.output().await.map_err(|e| {
        Error::Internal(format!("Failed to execute grep: {}", e))
    })?;

    // Exit code 1 means no matches, which is fine
    if !output.status.success() && output.status.code() != Some(1) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!("grep returned non-zero: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_grep_files_output(&stdout, &search_dir)
}

/// Parse standard grep -l output into workspace-relative paths
fn parse_grep_files_output(
    output: &str,
    search_dir: &str,
) -> Result<Vec<String>> {
    let mut files: Vec<String> = Vec::new();

    for line in output.lines() {
        let path = line.trim();
        if path.is_empty() {
            continue;
        }

        let relative_path = path.trim_start_matches("./");

        // Build workspace-relative path
        let workspace_relative = if search_dir.is_empty() {
            format!("/{}", relative_path)
        } else {
            format!("/{}/{}", search_dir.trim_end_matches('/'), relative_path)
        };

        files.push(workspace_relative);
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memory_path_user() {
        let path = "/users/00000000-0000-0000-0000-000000000001/memories/work/meeting.md";
        let result = parse_memory_path(path);
        assert!(result.is_some());
        let (scope, category, key) = result.unwrap();
        assert_eq!(scope, MemoryScope::User);
        assert_eq!(category, "work");
        assert_eq!(key, "meeting");
    }

    #[test]
    fn test_parse_memory_path_global() {
        let path = "/memories/config/settings.md";
        let result = parse_memory_path(path);
        assert!(result.is_some());
        let (scope, category, key) = result.unwrap();
        assert_eq!(scope, MemoryScope::Global);
        assert_eq!(category, "config");
        assert_eq!(key, "settings");
    }

    #[test]
    fn test_parse_memory_path_invalid() {
        assert!(parse_memory_path("/invalid/path").is_none());
        assert!(parse_memory_path("/memories/only-category").is_none());
    }

    #[test]
    fn test_extract_content_preview_short() {
        let body = "This is a short text.";
        let preview = extract_content_preview(body, 100);
        assert_eq!(preview, "This is a short text.");
    }

    #[test]
    fn test_extract_content_preview_truncated() {
        let body = "word ".repeat(150);
        let preview = extract_content_preview(&body, 100);
        assert!(preview.ends_with("..."));
        // Should have 100 words + "..."
        let word_count = preview.trim_end_matches('.').split_whitespace().count();
        assert_eq!(word_count, 100);
    }

    #[test]
    fn test_extract_content_preview_exact() {
        let body = "word ".repeat(100);
        let preview = extract_content_preview(&body.trim(), 100);
        assert!(!preview.ends_with("..."));
    }
}
