//! Memory delete tool - soft deletes memory files by scope, category, and key.
//!
//! Supports user-scoped (private) and global (shared) memory deletion.

use crate::error::{Error, Result};
use crate::models::requests::{ToolResponse, MemoryDeleteArgs, MemoryDeleteResult};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{generate_memory_path, MemoryScope};
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

pub struct MemoryDeleteTool;

#[async_trait]
impl Tool for MemoryDeleteTool {
    fn name(&self) -> &'static str {
        "memory_delete"
    }

    fn description(&self) -> &'static str {
        r#"Deletes a stored memory by scope, category, and key.

Performs a soft delete - the memory can be recovered from the deleted files view.

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
        let memory_args: MemoryDeleteArgs = serde_json::from_value(args)?;

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
                return Err(Error::NotFound(format!(
                    "Memory not found: {}/{}/{}",
                    memory_args.scope, memory_args.category, memory_args.key
                )));
            }
        }

        // Perform soft delete
        let file_id = file.id;
        files::soft_delete_file(conn, storage, file_id).await?;

        let result = MemoryDeleteResult {
            path,
            file_id: Some(file_id),
            scope: memory_args.scope,
            category: memory_args.category,
            key: memory_args.key,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
