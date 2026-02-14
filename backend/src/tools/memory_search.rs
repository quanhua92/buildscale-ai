//! Memory search tool - searches memory files with scope, category, and tag filtering.
//!
//! Uses regex for pattern matching and filters results based on memory metadata.

use crate::error::{Error, Result};
use crate::models::files::FileType;
use crate::models::requests::{
    ToolResponse, MemorySearchArgs, MemorySearchResult, MemoryMatch,
};
use crate::queries::files as file_queries;
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{parse_memory_frontmatter, MemoryScope};
use crate::DbConn;
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;
use tokio::fs;
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
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let search_args: MemorySearchArgs = serde_json::from_value(args)?;

        let limit = search_args.limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
        let case_sensitive = search_args.case_sensitive.unwrap_or(false);

        // Build regex pattern
        let pattern = if case_sensitive {
            Regex::new(&search_args.pattern)
        } else {
            Regex::new(&format!("(?i){}", search_args.pattern))
        }.map_err(|e| Error::Validation(crate::error::ValidationErrors::Single {
            field: "pattern".to_string(),
            message: format!("Invalid regex pattern: {}", e),
        }))?;

        // Get all memory files from database
        let memory_files = file_queries::get_files_by_type(conn, workspace_id, FileType::Memory).await?;

        let mut all_matches: Vec<MemoryMatch> = Vec::new();

        for file in memory_files {
            // Stop if we've reached the limit
            if limit > 0 && all_matches.len() >= limit {
                break;
            }

            // Parse scope, category, key from path
            let (scope, category, key) = match parse_memory_path(&file.path) {
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
                if !file.path.starts_with(&expected_prefix) {
                    continue; // Skip other users' memories
                }
            }

            // Apply category filter
            if let Some(ref filter_category) = search_args.category {
                if &category != filter_category {
                    continue;
                }
            }

            // Read file content from filesystem
            let workspace_path = storage.get_workspace_path(workspace_id);
            let file_path = workspace_path.join(file.path.trim_start_matches('/'));

            let content = match fs::read_to_string(&file_path).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(path = %file.path, error = %e, "Failed to read memory file");
                    continue;
                }
            };

            // Parse frontmatter to get metadata
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

            // Search for pattern in content
            for (line_idx, line) in body_content.lines().enumerate() {
                if limit > 0 && all_matches.len() >= limit {
                    break;
                }

                if pattern.is_match(line) {
                    let mem_metadata = metadata.clone().unwrap_or_else(|| crate::utils::MemoryMetadata {
                        title: key.clone(),
                        tags: vec![],
                        category: category.clone(),
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        scope: scope.clone(),
                    });

                    all_matches.push(MemoryMatch {
                        path: file.path.clone(),
                        scope: scope.clone(),
                        category: category.clone(),
                        key: key.clone(),
                        title: mem_metadata.title,
                        line_number: line_idx + 1,
                        line_text: line.to_string(),
                        tags: mem_metadata.tags,
                    });
                }
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
