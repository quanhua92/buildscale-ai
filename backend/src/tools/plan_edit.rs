//! Plan edit tool - wraps edit tool while preserving frontmatter.

use crate::error::{Error, Result, ValidationErrors};
use crate::models::files::FileType;
use crate::models::requests::{CreateVersionRequest, ToolResponse, PlanEditArgs, WriteResult};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{parse_frontmatter, prepend_frontmatter};
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

pub struct PlanEditTool;

#[async_trait]
impl Tool for PlanEditTool {
    fn name(&self) -> &'static str {
        "plan_edit"
    }

    fn description(&self) -> &'static str {
        r#"Edits a plan file while preserving YAML frontmatter.

Same as edit tool but:
- Preserves frontmatter during edits
- Only works on .plan files"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "old_string": {"type": ["string", "null"], "description": "For REPLACE: text to find (must be unique)"},
                "new_string": {"type": ["string", "null"], "description": "For REPLACE: replacement text"},
                "insert_line": {"type": ["integer", "string", "null"], "description": "For INSERT: line number (0-indexed). Accepts integer or string."},
                "insert_content": {"type": ["string", "null"], "description": "For INSERT: content to insert"},
                "last_read_hash": {"type": ["string", "null"], "description": "Hash from latest read"}
            },
            "required": ["path"],
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
        let plan_args: PlanEditArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&plan_args.path);

        // Ensure it's a .plan file
        if !path.ends_with(".plan") {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: "plan_edit only works on .plan files".to_string(),
            }));
        }

        // Get current file
        let file = file_queries::get_file_by_path(conn, workspace_id, &path).await?
            .ok_or_else(|| Error::NotFound(format!("Plan not found: {}", path)))?;

        // Plan Mode Guard
        if config.plan_mode && !matches!(file.file_type, FileType::Plan) {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: super::PLAN_MODE_ERROR.to_string(),
            }));
        }

        // Virtual File Protection
        if file.is_virtual {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: "Cannot edit a virtual file directly. Use specialized system tools.".to_string(),
            }));
        }

        // Folders cannot be edited
        if matches!(file.file_type, FileType::Folder) {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: "Cannot edit a folder.".to_string(),
            }));
        }

        // Get current content
        let file_with_content = files::get_file_with_content(conn, storage, file.id).await?;

        // Optional: Validate hash
        if let Some(last_read_hash) = &plan_args.last_read_hash {
            if &file_with_content.latest_version.hash != last_read_hash {
                return Err(Error::Conflict(format!(
                    "File content has changed. Expected hash: {}, but latest is: {}. Please read the file again.",
                    last_read_hash, file_with_content.latest_version.hash
                )));
            }
        }

        // Extract content as string
        let content_text = match &file_with_content.content {
            Value::String(s) => s.clone(),
            other => {
                if let Some(s) = other.as_str() {
                    s.to_string()
                } else if let Some(text) = other.get("text").and_then(|t| t.as_str()) {
                    text.to_string()
                } else {
                    other.to_string()
                }
            }
        };

        // Parse existing frontmatter
        let (existing_metadata, content_without_frontmatter) = parse_frontmatter(&content_text);

        // Determine operation type
        let is_replace = plan_args.old_string.is_some() && plan_args.new_string.is_some();
        let is_insert = plan_args.insert_line.is_some() && plan_args.insert_content.is_some();

        // Validation
        if !is_replace && !is_insert {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "operation".to_string(),
                message: "Must specify either (old_string + new_string) for Replace or (insert_line + insert_content) for Insert".to_string(),
            }));
        }

        if is_replace && is_insert {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "operation".to_string(),
                message: "Cannot specify both Replace and Insert operations.".to_string(),
            }));
        }

        // Perform the edit on content without frontmatter
        let edited_content = if is_replace {
            let old_string = plan_args.old_string.as_ref().unwrap();
            let new_string = plan_args.new_string.as_ref().unwrap();

            if old_string.is_empty() {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "old_string".to_string(),
                    message: "Search string cannot be empty".to_string(),
                }));
            }

            // Count matches
            let matches: Vec<_> = content_without_frontmatter.match_indices(old_string).collect();
            let count = matches.len();

            if count == 0 {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "old_string".to_string(),
                    message: "Search string not found in plan content".to_string(),
                }));
            }

            if count > 1 {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "old_string".to_string(),
                    message: format!("Search string found {} times. Provide more context for unique match.", count),
                }));
            }

            content_without_frontmatter.replacen(old_string, new_string, 1)
        } else {
            // Insert operation
            let insert_line = plan_args.insert_line.unwrap();
            let insert_content = plan_args.insert_content.as_ref().unwrap();

            if insert_content.is_empty() {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "insert_content".to_string(),
                    message: "Insert content cannot be empty".to_string(),
                }));
            }

            let mut lines: Vec<&str> = content_without_frontmatter.lines().collect();

            if insert_line > lines.len() {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "insert_line".to_string(),
                    message: format!("Insert line {} is out of bounds (file has {} lines)", insert_line, lines.len()),
                }));
            }

            lines.insert(insert_line, insert_content);
            lines.join("\n")
        };

        // Re-add frontmatter if it existed
        let final_content = if let Some(metadata) = existing_metadata {
            prepend_frontmatter(&metadata, &edited_content)
        } else {
            // No existing frontmatter, just use edited content
            edited_content.to_string()
        };

        // Create new version
        let version = files::create_version(conn, storage, file.id, CreateVersionRequest {
            author_id: Some(user_id),
            branch: Some("main".to_string()),
            content: serde_json::json!(final_content),
            app_data: None,
        }).await?;

        let result = WriteResult {
            path,
            file_id: file.id,
            version_id: version.id,
            hash: version.hash,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
