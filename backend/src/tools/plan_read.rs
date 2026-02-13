//! Plan read tool - wraps read tool with frontmatter parsing and name lookup.

use crate::error::{Error, Result, ValidationErrors};
use crate::models::requests::{ToolResponse, PlanReadArgs, PlanReadResult};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::parse_frontmatter;
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

/// Default maximum number of lines to read from a plan file.
const DEFAULT_READ_LIMIT: usize = 500;

pub struct PlanReadTool;

#[async_trait]
impl Tool for PlanReadTool {
    fn name(&self) -> &'static str {
        "plan_read"
    }

    fn description(&self) -> &'static str {
        r#"Reads a plan file with parsed frontmatter.

Supports lookup by path or name (searches /plans/ directory).
Returns metadata (title, status, created_at) and content."#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": ["string", "null"],
                    "description": "Full path to plan file"
                },
                "name": {
                    "type": ["string", "null"],
                    "description": "Plan name (without .plan), searches /plans/ directory"
                },
                "offset": {
                    "type": ["integer", "string", "null"],
                    "description": "Starting line position (passed to read tool). Accepts integer or string."
                },
                "limit": {
                    "type": ["integer", "string", "null"],
                    "description": "Maximum lines to read (default: 500). Accepts integer or string."
                },
                "cursor": {
                    "type": ["integer", "string", "null"],
                    "description": "Cursor position for scroll mode. Accepts integer or string."
                }
            },
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let plan_args: PlanReadArgs = serde_json::from_value(args)?;

        // Determine path
        let path = if let Some(p) = plan_args.path {
            p
        } else if let Some(name) = plan_args.name {
            format!("/plans/{}.plan", name)
        } else {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: "Either 'path' or 'name' is required".to_string(),
            }));
        };

        let path = super::normalize_path(&path);

        // Ensure it's a .plan file
        if !path.ends_with(".plan") {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: "plan_read only works on .plan files".to_string(),
            }));
        }

        // Get file from database
        let file = file_queries::get_file_by_path(conn, workspace_id, &path).await?
            .ok_or_else(|| Error::NotFound(format!("Plan not found: {}", path)))?;

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
        let (metadata, remaining_content) = parse_frontmatter(&content_text);

        // Apply offset and limit to remaining content
        let offset = plan_args.offset.unwrap_or(0);
        let limit = plan_args.limit.unwrap_or(DEFAULT_READ_LIMIT);

        // Handle cursor mode if provided
        let actual_offset = if let Some(cursor) = plan_args.cursor {
            if offset < 0 {
                // Scroll up from cursor
                let up_lines = offset.abs() as usize;
                if up_lines >= cursor { 0 } else { cursor - up_lines }
            } else {
                // Scroll down from cursor
                cursor + offset as usize
            }
        } else if offset < 0 {
            // Negative offset: from end
            let lines: Vec<&str> = remaining_content.lines().collect();
            let total = lines.len();
            let from_end = offset.abs() as usize;
            if from_end >= total { 0 } else { total - from_end }
        } else {
            offset as usize
        };

        // Slice content
        let lines: Vec<&str> = remaining_content.lines().collect();
        let total_lines = lines.len();
        let end = (actual_offset + limit).min(total_lines);
        let sliced_content: String = lines.get(actual_offset..end)
            .unwrap_or(&[])
            .join("\n");

        let result = PlanReadResult {
            path: path.clone(),
            metadata,
            content: sliced_content,
            hash: file_with_content.latest_version.hash,
            total_lines: Some(total_lines),
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
