//! Memory search tool - searches memory files with scope, category, and tag filtering.
//!
//! Uses pure grep for efficient pattern matching, then filters results based on memory metadata.

use crate::error::{Error, Result};
use crate::models::requests::{
    ToolResponse, MemorySearchArgs, MemorySearchResult, MemoryMatch,
};
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{parse_memory_frontmatter, MemoryScope};
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command as TokioCommand;
use uuid::Uuid;

/// Default maximum number of search results
const DEFAULT_SEARCH_LIMIT: usize = 50;

pub struct MemorySearchTool;

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &'static str {
        "memory_search"
    }

    fn description(&self) -> &'static str {
        r#"Searches stored memories by pattern with optional filtering.

Supports filtering by scope, category, and tags. Returns matching lines with metadata.

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

        // Build grep patterns based on scope filter
        let grep_patterns = build_grep_patterns(&search_args.scope, user_id);

        // Use grep to find files matching the pattern
        let grep_matches = run_grep_for_memories(
            &search_args.pattern,
            &grep_patterns,
            case_sensitive,
            &workspace_path,
        ).await?;

        // If no matches from grep, return early
        if grep_matches.is_empty() {
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

        // Process each file that matched grep
        for (file_path, grep_lines) in &grep_matches {
            // Stop if we've reached the limit
            if limit > 0 && all_matches.len() >= limit {
                break;
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

            // Read file content for frontmatter (to get tags)
            let full_path = workspace_path.join(file_path.trim_start_matches('/'));
            let content = match tokio::fs::read_to_string(&full_path).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(path = %file_path, error = %e, "Failed to read memory file");
                    continue;
                }
            };

            // Parse frontmatter to get metadata
            let (metadata, _body_content) = parse_memory_frontmatter(&content);

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
            let updated_at = match std::fs::metadata(&full_path) {
                Ok(meta) => {
                    meta.modified()
                        .ok()
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or_else(chrono::Utc::now)
                }
                Err(_) => chrono::Utc::now(),
            };

            // Build metadata for matches
            let mem_metadata = metadata.clone().unwrap_or_else(|| crate::utils::MemoryMetadata {
                title: key.clone(),
                tags: vec![],
                category: category.clone(),
                created_at: chrono::Utc::now(),
                updated_at,
                scope: scope.clone(),
            });

            // Create matches from grep lines
            for (line_number, line_text) in grep_lines {
                if limit > 0 && all_matches.len() >= limit {
                    break;
                }

                all_matches.push(MemoryMatch {
                    path: file_path.clone(),
                    scope: scope.clone(),
                    category: category.clone(),
                    key: key.clone(),
                    title: mem_metadata.title.clone(),
                    line_number: *line_number,
                    line_text: line_text.clone(),
                    tags: mem_metadata.tags.clone(),
                    updated_at,
                });
            }
        }

        // Sort by path, then line number
        all_matches.sort_by(|a, b| {
            a.path.cmp(&b.path).then_with(|| a.line_number.cmp(&b.line_number))
        });

        let result = MemorySearchResult {
            total: all_matches.len(),
            matches: all_matches,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

/// Build grep glob patterns based on scope filter
fn build_grep_patterns(
    scope_filter: &Option<MemoryScope>,
    user_id: Uuid,
) -> Vec<String> {
    match scope_filter {
        Some(MemoryScope::User) => {
            // Only search user's memory directory
            vec![format!("users/{}/memories/**/*.md", user_id)]
        }
        Some(MemoryScope::Global) => {
            // Only search global memory directory
            vec!["memories/**/*.md".to_string()]
        }
        None => {
            // Search both user and global directories
            vec![
                "memories/**/*.md".to_string(),
                format!("users/{}/memories/**/*.md", user_id),
            ]
        }
    }
}

/// Run grep on memory directories and return matches grouped by file path
async fn run_grep_for_memories(
    pattern: &str,
    glob_patterns: &[String],
    case_sensitive: bool,
    workspace_path: &Path,
) -> Result<HashMap<String, Vec<(usize, String)>>> {
    let mut all_matches: HashMap<String, Vec<(usize, String)>> = HashMap::new();

    for glob_pattern in glob_patterns {
        // Try ripgrep first, then fall back to standard grep
        let matches = run_ripgrep_glob(pattern, glob_pattern, case_sensitive, workspace_path).await;
        let matches = match matches {
            Ok(m) => m,
            Err(_) => run_standard_grep_glob(pattern, glob_pattern, case_sensitive, workspace_path).await?,
        };

        // Merge matches into result
        for (file_path, lines) in matches {
            all_matches.entry(file_path).or_insert_with(Vec::new).extend(lines);
        }
    }

    Ok(all_matches)
}

/// Run ripgrep with glob pattern
async fn run_ripgrep_glob(
    pattern: &str,
    glob_pattern: &str,
    case_sensitive: bool,
    workspace_path: &Path,
) -> Result<HashMap<String, Vec<(usize, String)>>> {
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
        return Ok(HashMap::new());
    }

    let mut cmd = TokioCommand::new("rg");
    cmd.current_dir(&search_path);
    cmd.arg("--json");
    cmd.arg("--line-number");
    cmd.arg("--glob");
    cmd.arg("*.md");

    if case_sensitive {
        cmd.arg("--case-sensitive");
    } else {
        cmd.arg("--ignore-case");
    }

    cmd.arg(pattern);
    cmd.arg(".");

    tracing::debug!("Running ripgrep with pattern: {}, search_dir: {}, search_path: {:?}", pattern, search_dir, search_path);

    let output = cmd.output().await.map_err(|e| {
        tracing::error!("Failed to execute ripgrep: {}", e);
        Error::Internal(format!("Failed to execute ripgrep: {}", e))
    })?;

    tracing::debug!("ripgrep exit code: {:?}", output.status.code());
    let stdout_len = String::from_utf8_lossy(&output.stdout).len();
    tracing::debug!("ripgrep stdout length: {}", stdout_len);
    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!("ripgrep stderr: {}", stderr);
    }

    // Exit code 1 means no matches, which is fine
    if !output.status.success() && output.status.code() != Some(1) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!("ripgrep returned non-zero: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ripgrep_json_output_with_prefix(&stdout, &search_dir)
}

/// Parse ripgrep JSON output with a prefix directory (for when searching in subdirectories)
fn parse_ripgrep_json_output_with_prefix(
    output: &str,
    search_dir: &str,
) -> Result<HashMap<String, Vec<(usize, String)>>> {
    let mut matches: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    let mut current_file: Option<String> = None;

    for line in output.lines() {
        if let Ok(event) = serde_json::from_str::<RipgrepEvent>(line) {
            match event {
                RipgrepEvent::Begin { data } => {
                    let path = &data.path.text;
                    // Convert relative path from search_dir to workspace-relative path
                    // path is relative to search_dir, so we prepend search_dir
                    let workspace_relative = if search_dir.is_empty() {
                        format!("/{}", path.trim_start_matches("./"))
                    } else {
                        format!("/{}/{}", search_dir.trim_end_matches('/'), path.trim_start_matches("./"))
                    };
                    current_file = Some(workspace_relative);
                }
                RipgrepEvent::Match { data } => {
                    if let Some(ref file_path) = current_file {
                        let line_number = data.line_number.unwrap_or(0) as usize;
                        let line_text = data.lines.text.trim_end().to_string();
                        matches.entry(file_path.clone()).or_default().push((line_number, line_text));
                    }
                }
                _ => {}
            }
        }
    }

    Ok(matches)
}

/// Run standard grep with glob pattern (directory-based)
async fn run_standard_grep_glob(
    pattern: &str,
    glob_pattern: &str,
    case_sensitive: bool,
    workspace_path: &Path,
) -> Result<HashMap<String, Vec<(usize, String)>>> {
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
        return Ok(HashMap::new());
    }

    let mut cmd = TokioCommand::new("grep");
    cmd.current_dir(&search_path);
    cmd.arg("-R");
    cmd.arg("-n");
    cmd.arg("-H");
    cmd.arg("--include=*.md");

    if !case_sensitive {
        cmd.arg("-i");
    }

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
    parse_grep_output(&stdout, &search_dir)
}

/// Parse standard grep output and convert to workspace-relative paths
fn parse_grep_output(
    output: &str,
    search_dir: &str,
) -> Result<HashMap<String, Vec<(usize, String)>>> {
    let mut matches: HashMap<String, Vec<(usize, String)>> = HashMap::new();

    for line in output.lines() {
        // Format: path:line_number:content
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() >= 3 {
            let relative_path = parts[0].trim_start_matches("./");
            let line_number: usize = parts[1].parse().unwrap_or(0);
            let line_text = parts[2].to_string();

            // Build workspace-relative path
            let workspace_relative = if search_dir.is_empty() {
                format!("/{}", relative_path)
            } else {
                format!("/{}/{}", search_dir.trim_end_matches('/'), relative_path)
            };

            matches.entry(workspace_relative).or_default().push((line_number, line_text));
        }
    }

    Ok(matches)
}

/// Parse scope, category, and key from memory path
fn parse_memory_path(path: &str) -> Option<(MemoryScope, String, String)> {
    // User path: /users/{user_id}/memories/{category}/{key}.md
    // Global path: /memories/{category}/{key}.md

    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() >= 6 && parts[1] == "users" && parts[3] == "memories" {
        // User-scoped memory: /users/{uuid}/memories/{category}/{key}.md
        let category = parts[4].to_string();
        let key = parts.get(5)?.strip_suffix(".md")?.to_string();
        Some((MemoryScope::User, category, key))
    } else if parts.len() >= 4 && parts[1] == "memories" {
        // Global-scoped memory: /memories/{category}/{key}.md
        let category = parts[2].to_string();
        let key = parts.get(3)?.strip_suffix(".md")?.to_string();
        Some((MemoryScope::Global, category, key))
    } else {
        None
    }
}

/// Ripgrep JSON event types
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RipgrepEvent {
    Begin { data: BeginData },
    Match { data: MatchData },
    #[allow(dead_code)]
    Context { data: ContextData },
    #[allow(dead_code)]
    End { data: EndData },
    #[allow(dead_code)]
    Summary { data: SummaryData },
}

#[derive(Debug, serde::Deserialize)]
struct BeginData {
    path: PathField,
}

#[derive(Debug, serde::Deserialize)]
struct MatchData {
    #[allow(dead_code)]
    path: PathField,
    lines: LinesField,
    line_number: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
struct ContextData {
    #[allow(dead_code)]
    path: PathField,
    #[allow(dead_code)]
    lines: LinesField,
    #[allow(dead_code)]
    line_number: u64,
}

#[derive(Debug, serde::Deserialize)]
struct EndData {
    #[allow(dead_code)]
    path: PathField,
}

#[derive(Debug, serde::Deserialize)]
struct SummaryData {
    #[allow(dead_code)]
    elapsed_total: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct PathField {
    text: String,
}

#[derive(Debug, serde::Deserialize)]
struct LinesField {
    text: String,
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
}
