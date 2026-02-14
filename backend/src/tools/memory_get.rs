//! Memory get tool - retrieves memory files by scope, category, and key.
//!
//! Supports user-scoped (private) and global (shared) memory retrieval.

use crate::error::{Error, Result};
use crate::models::requests::{ToolResponse, MemoryGetArgs, MemoryGetResult};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{generate_memory_path, parse_memory_frontmatter, MemoryScope};
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

pub struct MemoryGetTool;

#[async_trait]
impl Tool for MemoryGetTool {
    fn name(&self) -> &'static str {
        "memory_get"
    }

    fn description(&self) -> &'static str {
        r#"Retrieves a stored memory by scope, category, and key.

Returns the memory content with metadata (title, tags, timestamps).

Example: {"scope": "user", "category": "preferences", "key": "coding-style"}"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "scope": {
                    "type": "string",
                    "enum": ["user", "global"],
                    "description": "Memory scope: 'user' (private) or 'global' (shared)"
                },
                "category": {
                    "type": "string",
                    "description": "Category the memory belongs to"
                },
                "key": {
                    "type": "string",
                    "description": "Unique key for the memory within its category"
                }
            },
            "required": ["scope", "category", "key"],
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
        let memory_args: MemoryGetArgs = serde_json::from_value(args)?;

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

        // Get file from database
        let file = file_queries::get_file_by_path(conn, workspace_id, &path).await?
            .ok_or_else(|| Error::NotFound(format!("Memory not found: {}/{}/{}",
                memory_args.scope, memory_args.category, memory_args.key)))?;

        // For user-scoped memories, verify ownership
        if matches!(memory_args.scope, MemoryScope::User) {
            let expected_prefix = format!("/users/{}/memories/", user_id);
            if !file.path.starts_with(&expected_prefix) {
                return Err(Error::Forbidden(
                    "Cannot access another user's memory".to_string()
                ));
            }
        }

        // Get file content
        let file_with_content = files::get_file_with_content(conn, storage, file.id).await?;

        // Extract content as string
        let content_text = match &file_with_content.content {
            Value::String(s) => s.clone(),
            other => {
                // Try to extract text from JSON
                if let Some(s) = other.as_str() {
                    s.to_string()
                } else if let Some(text) = other.get("text").and_then(|t| t.as_str()) {
                    text.to_string()
                } else {
                    other.to_string()
                }
            }
        };

        // Parse frontmatter
        let (metadata, remaining_content) = parse_memory_frontmatter(&content_text);

        let result = MemoryGetResult {
            path: path.clone(),
            scope: memory_args.scope,
            category: memory_args.category,
            key: memory_args.key,
            metadata,
            content: remaining_content.to_string(),
            hash: file_with_content.latest_version.hash,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
