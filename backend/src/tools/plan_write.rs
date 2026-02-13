//! Plan write tool - wraps write tool with auto-naming and frontmatter.

use crate::error::{Error, Result, ValidationErrors};
use crate::models::files::FileType;
use crate::models::requests::{CreateFileRequest, CreateVersionRequest, ToolResponse, PlanWriteArgs, PlanWriteResult};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{generate_plan_name, PlanMetadata, PlanStatus, prepend_frontmatter};
use crate::DbConn;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;

pub struct PlanWriteTool;

#[async_trait]
impl Tool for PlanWriteTool {
    fn name(&self) -> &'static str {
        "plan_write"
    }

    fn description(&self) -> &'static str {
        r##"Creates or updates a plan file with auto-generated name and YAML frontmatter.

If path not provided, auto-generates a unique 3-word hyphenated name.
Automatically adds YAML frontmatter with title, status, and created_at.
Sets file_type to "plan" automatically.

Example: {"title": "Feature Plan", "content": "# Plan content here"}
Result: Creates /plans/gleeful-tangerine-expedition.plan"##
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Title of the plan (shown in frontmatter)"
                },
                "content": {
                    "type": "string",
                    "description": "Plan content in markdown"
                },
                "path": {
                    "type": ["string", "null"],
                    "description": "Optional path. If omitted, auto-generates a name like /plans/word-word-word.plan"
                },
                "status": {
                    "type": ["string", "null"],
                    "description": "Plan status: draft (default), approved, implemented, archived"
                }
            },
            "required": ["title", "content"],
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
        let plan_args: PlanWriteArgs = serde_json::from_value(args)?;

        // Determine path
        let path = if let Some(p) = plan_args.path {
            // Ensure it's in /plans/ and ends with .plan
            let normalized = if p.starts_with("/plans/") {
                p
            } else if p.starts_with('/') {
                format!("/plans{}", p)
            } else {
                format!("/plans/{}", p)
            };

            if !normalized.ends_with(".plan") {
                format!("{}.plan", normalized)
            } else {
                normalized
            }
        } else {
            // Auto-generate name with collision retry (max 5 attempts)
            let mut attempts = 0;
            let max_attempts = 5;
            let mut chosen_path: Option<String> = None;

            while attempts < max_attempts {
                let name = generate_plan_name();
                let candidate_path = format!("/plans/{}.plan", name);
                let candidate_path = super::normalize_path(&candidate_path);

                // Check if file already exists
                let existing = file_queries::get_file_by_path(conn, workspace_id, &candidate_path).await?;
                if existing.is_none() {
                    // Path is available, use it
                    chosen_path = Some(candidate_path);
                    break;
                }

                attempts += 1;
                tracing::debug!(
                    attempt = attempts,
                    candidate = %candidate_path,
                    "Generated plan name collision, retrying"
                );
            }

            match chosen_path {
                Some(p) => p,
                None => {
                    return Err(Error::Validation(ValidationErrors::Single {
                        field: "path".to_string(),
                        message: format!(
                            "Failed to generate unique plan name after {} attempts. Please provide a custom path.",
                            max_attempts
                        ),
                    }));
                }
            }
        };

        let path = super::normalize_path(&path);

        // Parse status
        let status = if let Some(s) = plan_args.status {
            PlanStatus::from_str(&s.to_lowercase()).unwrap_or(PlanStatus::Draft)
        } else {
            PlanStatus::Draft
        };

        // Create metadata and prepend frontmatter
        let metadata = PlanMetadata {
            title: plan_args.title,
            status,
            created_at: Utc::now(),
        };
        let content_with_frontmatter = prepend_frontmatter(&metadata, &plan_args.content);

        // Check if file exists
        let existing_file = file_queries::get_file_by_path(conn, workspace_id, &path).await?;

        // Plan Mode Guard: Only allow Plan files in plan mode
        if config.plan_mode {
            let is_plan_file = if let Some(ref file) = existing_file {
                matches!(file.file_type, FileType::Plan)
            } else {
                path.ends_with(".plan")
            };

            if !is_plan_file {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "path".to_string(),
                    message: super::PLAN_MODE_ERROR.to_string(),
                }));
            }
        }

        // Virtual File Protection
        if let Some(ref file) = existing_file {
            if file.is_virtual {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "path".to_string(),
                    message: "Cannot write to a virtual file directly. Use specialized system tools to modify this resource.".to_string(),
                }));
            }
        }

        let result = if let Some(file) = existing_file {
            // Update existing file
            let version = files::create_version(conn, storage, file.id, CreateVersionRequest {
                author_id: Some(user_id),
                branch: Some("main".to_string()),
                content: serde_json::json!(content_with_frontmatter),
                app_data: None,
            }).await?;

            PlanWriteResult {
                path: path.clone(),
                file_id: file.id,
                version_id: version.id,
                hash: version.hash,
                metadata,
            }
        } else {
            // Create new file
            let filename = path.rsplit('/').next().unwrap_or("untitled");

            let file_result = files::create_file_with_content(conn, storage, CreateFileRequest {
                workspace_id,
                parent_id: None,
                author_id: user_id,
                name: filename.to_string(),
                slug: None,
                path: Some(path.clone()),
                is_virtual: None,
                is_remote: None,
                permission: None,
                file_type: FileType::Plan,
                content: serde_json::json!(content_with_frontmatter),
                app_data: None,
            }).await?;

            PlanWriteResult {
                path,
                file_id: file_result.file.id,
                version_id: file_result.latest_version.id,
                hash: file_result.latest_version.hash,
                metadata,
            }
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
