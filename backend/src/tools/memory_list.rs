//! Memory list tool - efficiently lists categories, tags, or memories without full content.
//!
//! Provides efficient listing operations:
//! - `categories`: Directory scan to find unique category names
//! - `tags`: Parse frontmatter only to extract tag counts
//! - `memories`: List memory metadata without content preview

use crate::error::{Error, Result};
use crate::models::requests::{
    ToolResponse, MemoryListArgs, MemoryListType,
    CategoryInfo, TagInfo, MemoryListItem,
    MemoryListCategoriesResult, MemoryListTagsResult, MemoryListMemoriesResult,
};
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{parse_memory_frontmatter, parse_memory_path, MemoryScope};
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct MemoryListTool;

#[async_trait]
impl Tool for MemoryListTool {
    fn name(&self) -> &'static str {
        "memory_list"
    }

    fn description(&self) -> &'static str {
        r#"Efficiently lists categories, tags, or memories without loading full content.

Use this tool to:
- Get all unique categories with memory counts
- Get all unique tags with usage counts
- List memories with metadata only (no content preview)

Examples:
- List categories: {"list_type": "categories"}
- List tags in user scope: {"list_type": "tags", "scope": "user"}
- List memories in a category: {"list_type": "memories", "category": "preferences"}"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "list_type": {
                    "type": "string",
                    "enum": ["categories", "tags", "memories"],
                    "description": "Type of listing: 'categories', 'tags', or 'memories'"
                },
                "scope": {
                    "type": ["string", "null"],
                    "enum": ["user", "global", null],
                    "description": "Filter by scope: 'user' or 'global'"
                },
                "category": {
                    "type": ["string", "null"],
                    "description": "Filter by category (for tags/memories listing)"
                },
                "tags": {
                    "type": ["array", "null"],
                    "items": {"type": "string"},
                    "description": "Filter by tags (for memories listing, must have ALL tags)"
                },
                "limit": {
                    "type": ["integer", "string", "null"],
                    "description": "Maximum results to return (default: 100)"
                },
                "offset": {
                    "type": ["integer", "string", "null"],
                    "description": "Offset for pagination (default: 0)"
                }
            },
            "required": ["list_type"],
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
        let list_args: MemoryListArgs = serde_json::from_value(args)?;

        let workspace_path = storage.get_workspace_path(workspace_id);

        let result = match list_args.list_type {
            MemoryListType::Categories => {
                let (categories, total) = list_categories(
                    &workspace_path,
                    list_args.scope.as_ref(),
                    user_id,
                    list_args.limit,
                    list_args.offset,
                ).await?;
                serde_json::to_value(MemoryListCategoriesResult { categories, total })?
            }
            MemoryListType::Tags => {
                let (tags, total) = list_tags(
                    &workspace_path,
                    list_args.scope.as_ref(),
                    list_args.category.as_deref(),
                    user_id,
                    list_args.limit,
                    list_args.offset,
                ).await?;
                serde_json::to_value(MemoryListTagsResult { tags, total })?
            }
            MemoryListType::Memories => {
                let (memories, total) = list_memories(
                    &workspace_path,
                    list_args.scope.as_ref(),
                    list_args.category.as_deref(),
                    list_args.tags.as_ref(),
                    user_id,
                    list_args.limit,
                    list_args.offset,
                ).await?;
                serde_json::to_value(MemoryListMemoriesResult { memories, total })?
            }
        };

        Ok(ToolResponse {
            success: true,
            result,
            error: None,
        })
    }
}

/// List unique categories with memory counts via directory scan
async fn list_categories(
    workspace_path: &Path,
    scope_filter: Option<&MemoryScope>,
    user_id: Uuid,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<(Vec<CategoryInfo>, usize)> {
    let mut category_counts: HashMap<String, usize> = HashMap::new();

    // Scan global memories if no scope filter or global scope
    if scope_filter.is_none() || matches!(scope_filter, Some(MemoryScope::Global)) {
        let global_memories_path = workspace_path.join("memories");
        if global_memories_path.exists() {
            scan_categories_from_dir(&global_memories_path, &mut category_counts).await?;
        }
    }

    // Scan user memories if no scope filter or user scope
    if scope_filter.is_none() || matches!(scope_filter, Some(MemoryScope::User)) {
        let user_memories_path = workspace_path.join("users").join(user_id.to_string()).join("memories");
        if user_memories_path.exists() {
            scan_categories_from_dir(&user_memories_path, &mut category_counts).await?;
        }
    }

    // Convert to sorted vector
    let mut categories: Vec<CategoryInfo> = category_counts
        .into_iter()
        .map(|(name, count)| CategoryInfo { name, count })
        .collect();
    categories.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

    // Get total before pagination
    let total = categories.len();

    // Apply pagination
    apply_pagination(&mut categories, limit, offset);

    Ok((categories, total))
}

/// Scan a directory for category folders and count files
async fn scan_categories_from_dir(
    dir: &Path,
    category_counts: &mut HashMap<String, usize>,
) -> Result<()> {
    let mut entries = tokio::fs::read_dir(dir).await
        .map_err(|e| Error::Internal(format!("Failed to read directory: {}", e)))?;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| Error::Internal(format!("Failed to read entry: {}", e)))?
    {
        let path = entry.path();
        if path.is_dir() {
            if let Some(category_name) = path.file_name().and_then(|n| n.to_str()) {
                // Count .md files in this category directory
                let count = count_md_files_in_dir(&path).await?;
                if count > 0 {
                    *category_counts.entry(category_name.to_string()).or_insert(0) += count;
                }
            }
        }
    }

    Ok(())
}

/// Count .md files in a directory (non-recursive)
async fn count_md_files_in_dir(dir: &Path) -> Result<usize> {
    let mut count = 0;
    let mut entries = tokio::fs::read_dir(dir).await
        .map_err(|e| Error::Internal(format!("Failed to read directory: {}", e)))?;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| Error::Internal(format!("Failed to read entry: {}", e)))?
    {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            count += 1;
        }
    }

    Ok(count)
}

/// List unique tags with usage counts
async fn list_tags(
    workspace_path: &Path,
    scope_filter: Option<&MemoryScope>,
    category_filter: Option<&str>,
    user_id: Uuid,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<(Vec<TagInfo>, usize)> {
    let mut tag_counts: HashMap<String, usize> = HashMap::new();

    // Collect files from global memories
    if scope_filter.is_none() || matches!(scope_filter, Some(MemoryScope::Global)) {
        let global_memories_path = workspace_path.join("memories");
        let files = collect_memory_files(&global_memories_path, category_filter).await?;
        for file_path in files {
            if let Ok(content) = read_file_head(&file_path).await {
                let (metadata, _) = parse_memory_frontmatter(&content);
                if let Some(mem_metadata) = metadata {
                    for tag in mem_metadata.tags {
                        *tag_counts.entry(tag).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // Collect files from user memories
    if scope_filter.is_none() || matches!(scope_filter, Some(MemoryScope::User)) {
        let user_memories_path = workspace_path.join("users").join(user_id.to_string()).join("memories");
        let files = collect_memory_files(&user_memories_path, category_filter).await?;
        for file_path in files {
            if let Ok(content) = read_file_head(&file_path).await {
                let (metadata, _) = parse_memory_frontmatter(&content);
                if let Some(mem_metadata) = metadata {
                    for tag in mem_metadata.tags {
                        *tag_counts.entry(tag).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // Convert to sorted vector
    let mut tags: Vec<TagInfo> = tag_counts
        .into_iter()
        .map(|(name, count)| TagInfo { name, count })
        .collect();
    tags.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

    // Get total before pagination
    let total = tags.len();

    // Apply pagination
    apply_pagination(&mut tags, limit, offset);

    Ok((tags, total))
}

/// List memories with metadata (no content)
async fn list_memories(
    workspace_path: &Path,
    scope_filter: Option<&MemoryScope>,
    category_filter: Option<&str>,
    tags_filter: Option<&Vec<String>>,
    user_id: Uuid,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<(Vec<MemoryListItem>, usize)> {
    let mut memories: Vec<MemoryListItem> = Vec::new();

    // Collect and process files based on scope filter
    let paths_to_scan: Vec<(PathBuf, MemoryScope)> = match scope_filter {
        Some(MemoryScope::Global) => {
            vec![(workspace_path.join("memories"), MemoryScope::Global)]
        }
        Some(MemoryScope::User) => {
            vec![(
                workspace_path.join("users").join(user_id.to_string()).join("memories"),
                MemoryScope::User,
            )]
        }
        None => {
            vec![
                (workspace_path.join("memories"), MemoryScope::Global),
                (
                    workspace_path.join("users").join(user_id.to_string()).join("memories"),
                    MemoryScope::User,
                ),
            ]
        }
    };

    for (memories_path, default_scope) in paths_to_scan {
        let files = collect_memory_files(&memories_path, category_filter).await?;

        for file_path in files {
            // Parse path to extract category and key
            let relative_path = file_path.strip_prefix(workspace_path)
                .map_err(|e| Error::Internal(format!("Failed to strip prefix: {}", e)))?;
            let path_str = relative_path.to_string_lossy();

            let (scope, category, key) = match parse_memory_path(&format!("/{}", path_str)) {
                Some(result) => result,
                None => continue,
            };

            // Verify scope matches expected
            if scope != default_scope {
                continue;
            }

            // For user-scoped memories, verify ownership
            if scope == MemoryScope::User {
                let expected_prefix = format!("users/{}/memories/", user_id);
                if !path_str.starts_with(&expected_prefix) {
                    continue; // Skip other users' memories
                }
            }

            // Read and parse frontmatter (only first 4KB for efficiency)
            let content = match read_file_head(&file_path).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            let (metadata, _) = parse_memory_frontmatter(&content);

            // Apply tags filter
            if let Some(filter_tags) = tags_filter {
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
            let updated_at = match tokio::fs::metadata(&file_path).await {
                Ok(meta) => {
                    meta.modified()
                        .ok()
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                        .unwrap_or_else(chrono::Utc::now)
                }
                Err(_) => chrono::Utc::now(),
            };

            // Build metadata
            let mem_metadata = metadata.clone().unwrap_or_else(|| crate::utils::MemoryMetadata {
                title: key.clone(),
                tags: vec![],
                category: category.clone(),
                created_at: chrono::Utc::now(),
                updated_at,
                scope: scope.clone(),
            });

            memories.push(MemoryListItem {
                path: format!("/{}", path_str),
                scope,
                category,
                key,
                title: mem_metadata.title,
                tags: mem_metadata.tags,
                updated_at,
            });
        }
    }

    // Sort by updated_at descending
    memories.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    // Get total before pagination
    let total = memories.len();

    // Apply pagination
    let offset_val = offset.unwrap_or(0);
    let limit_val = limit.unwrap_or(100);

    if offset_val > 0 && offset_val < total {
        memories = memories.into_iter().skip(offset_val).collect();
    }

    if limit_val > 0 && memories.len() > limit_val {
        memories.truncate(limit_val);
    }

    Ok((memories, total))
}

/// Read only the beginning of a file for frontmatter parsing (more efficient than reading entire file)
/// Frontmatter is typically at the start, so 4KB should be sufficient
async fn read_file_head(path: &Path) -> Result<String> {
    use tokio::io::{AsyncReadExt, BufReader};

    let file = tokio::fs::File::open(path).await
        .map_err(|e| Error::Internal(format!("Failed to open file: {}", e)))?;

    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; 4096]; // Read first 4KB

    let bytes_read = reader.read(&mut buffer).await
        .map_err(|e| Error::Internal(format!("Failed to read file: {}", e)))?;

    buffer.truncate(bytes_read);

    String::from_utf8(buffer)
        .map_err(|e| Error::Internal(format!("Invalid UTF-8 in file: {}", e)))
}

/// Collect memory files from a directory (non-recursive for category, recursive for all)
async fn collect_memory_files(
    base_path: &Path,
    category: Option<&str>,
) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    if let Some(cat) = category {
        // Specific category: just scan that directory
        let cat_path = base_path.join(cat);
        if cat_path.exists() {
            scan_md_files(&cat_path, &mut files).await?;
        }
    } else {
        // All categories: scan subdirectories
        if base_path.exists() {
            let mut entries = tokio::fs::read_dir(base_path).await
                .map_err(|e| Error::Internal(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| Error::Internal(format!("Failed to read entry: {}", e)))?
            {
                let path = entry.path();
                if path.is_dir() {
                    scan_md_files(&path, &mut files).await?;
                }
            }
        }
    }

    Ok(files)
}

/// Scan .md files in a single directory
async fn scan_md_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
    let mut entries = tokio::fs::read_dir(dir).await
        .map_err(|e| Error::Internal(format!("Failed to read directory: {}", e)))?;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| Error::Internal(format!("Failed to read entry: {}", e)))?
    {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            files.push(path);
        }
    }

    Ok(())
}

/// Apply pagination to a list
fn apply_pagination<T>(list: &mut Vec<T>, limit: Option<usize>, offset: Option<usize>) {
    let offset_val = offset.unwrap_or(0);

    // Handle offset: skip first N items
    if offset_val > 0 {
        if offset_val >= list.len() {
            list.clear();
        } else {
            list.drain(..offset_val);
        }
    }

    // Handle limit: take only first M items
    let limit_val = limit.unwrap_or(100);
    if limit_val > 0 && list.len() > limit_val {
        list.truncate(limit_val);
    }
}
