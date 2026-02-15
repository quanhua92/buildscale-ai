//! Memory set tool - creates or updates memory files with auto-generated paths.
//!
//! Memory files store persistent AI agent memories with YAML frontmatter.
//! They support two scopes:
//! - User scope: Private to a specific user (`/users/{user_id}/memories/{category}/{key}.md`)
//! - Global scope: Shared across workspace (`/memories/{category}/{key}.md`)

use crate::error::{Error, Result, ValidationErrors};
use crate::models::files::FileType;
use crate::models::requests::{
    CreateFileRequest, CreateVersionRequest, ToolResponse, MemorySetArgs, MemorySetResult,
};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{
    generate_memory_path, prepend_memory_frontmatter, MemoryMetadata, MemoryScope,
};
use crate::DbConn;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

pub struct MemorySetTool;

#[async_trait]
impl Tool for MemorySetTool {
    fn name(&self) -> &'static str {
        "memory_set"
    }

    fn description(&self) -> &'static str {
        r##"Stores a memory with metadata for later retrieval.

Memories are persistent storage for AI agents with two scopes:
- user: Private to you, stored in /users/{your_id}/memories/{category}/{key}.md
- global: Shared with workspace, stored in /memories/{category}/{key}.md

Use cases:
- Remember user preferences, project context, decisions
- Store reusable information across sessions
- Share knowledge with other agents (global scope)

Example: {"scope": "user", "category": "preferences", "key": "coding-style", "title": "Coding Style Preferences", "content": "User prefers TypeScript with strict mode...", "tags": ["coding", "typescript"]}"##
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "scope": {
                    "type": "string",
                    "enum": ["user", "global"],
                    "description": "Memory visibility: 'user' (private) or 'global' (shared)"
                },
                "category": {
                    "type": "string",
                    "description": "Category for organization (e.g., 'preferences', 'project', 'decisions')"
                },
                "key": {
                    "type": "string",
                    "description": "Unique identifier within category (e.g., 'coding-style', 'api-keys')"
                },
                "title": {
                    "type": "string",
                    "description": "Human-readable title for the memory"
                },
                "content": {
                    "type": "string",
                    "description": "Memory content in markdown format"
                },
                "tags": {
                    "type": ["array", "null"],
                    "items": {"type": "string"},
                    "description": "Optional tags for categorization and search"
                }
            },
            "required": ["scope", "category", "key", "title", "content"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let memory_args: MemorySetArgs = serde_json::from_value(args)?;

        // Validate required fields
        if memory_args.category.trim().is_empty() {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "category".to_string(),
                message: "category cannot be empty".to_string(),
            }));
        }
        if memory_args.key.trim().is_empty() {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "key".to_string(),
                message: "key cannot be empty".to_string(),
            }));
        }
        if memory_args.title.trim().is_empty() {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "title".to_string(),
                message: "title cannot be empty".to_string(),
            }));
        }

        // Generate path based on scope
        let user_id_for_path = if matches!(memory_args.scope, MemoryScope::User) {
            Some(user_id)
        } else {
            None
        };

        let path = generate_memory_path(
            &memory_args.scope,
            &memory_args.category,
            &memory_args.key,
            user_id_for_path,
        );

        let path = super::normalize_path(&path);

        // Check if file exists for update tracking
        let existing_file = file_queries::get_file_by_path(conn, workspace_id, &path).await?;

        // For user-scoped memories, ensure user can only access their own memories
        if let Some(ref file) = existing_file {
            // Verify user owns user-scoped memories
            if matches!(memory_args.scope, MemoryScope::User) {
                // The path should contain the user_id for user-scoped memories
                let expected_prefix = format!("/users/{}/memories/", user_id);
                if !file.path.starts_with(&expected_prefix) {
                    return Err(Error::Forbidden(
                        "Cannot modify another user's memory".to_string()
                    ));
                }
            }
        }

        // Memory tools are allowed in plan mode for context persistence

        // Virtual File Protection
        if let Some(ref file) = existing_file {
            if file.is_virtual {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "path".to_string(),
                    message: "Cannot write to a virtual file directly.".to_string(),
                }));
            }
        }

        // Preserve original created_at from database when updating
        let old_created_at = existing_file.as_ref().map(|f| f.created_at);

        // Create metadata
        let now = Utc::now();
        let tags = memory_args.tags.clone().unwrap_or_default();
        let metadata = MemoryMetadata {
            title: memory_args.title.clone(),
            tags: tags.clone(),
            category: memory_args.category.clone(),
            created_at: old_created_at.unwrap_or(now),
            updated_at: now,
            scope: memory_args.scope.clone(),
        };

        // Prepend frontmatter to content
        let content_with_frontmatter = prepend_memory_frontmatter(&metadata, &memory_args.content);

        // Create or update file
        let result = if let Some(file) = existing_file {
            // Update existing file
            let version = files::create_version(conn, storage, file.id, CreateVersionRequest {
                author_id: Some(user_id),
                branch: Some("main".to_string()),
                content: serde_json::json!(content_with_frontmatter),
                app_data: None,
            }).await?;

            MemorySetResult {
                path: path.clone(),
                file_id: file.id,
                version_id: version.id,
                hash: version.hash,
                scope: memory_args.scope,
                category: memory_args.category,
                key: memory_args.key,
                title: memory_args.title,
                tags,
            }
        } else {
            // Create new file
            let filename = format!("{}.md", memory_args.key);

            let file_result = files::create_file_with_content(conn, storage, CreateFileRequest {
                workspace_id,
                parent_id: None,
                author_id: user_id,
                name: filename,
                slug: None,
                path: Some(path.clone()),
                is_virtual: None,
                is_remote: None,
                permission: None,
                file_type: FileType::Memory,
                content: serde_json::json!(content_with_frontmatter),
                app_data: None,
            }).await?;

            MemorySetResult {
                path,
                file_id: file_result.file.id,
                version_id: file_result.latest_version.id,
                hash: file_result.latest_version.hash,
                scope: memory_args.scope,
                category: memory_args.category,
                key: memory_args.key,
                title: memory_args.title,
                tags,
            }
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
