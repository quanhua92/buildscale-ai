//! Plan list tool - lists plan files with metadata.

use crate::error::Result;
use crate::models::files::FileType;
use crate::models::requests::{ToolResponse, PlanListArgs, PlanListResult, PlanListItem};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::tools::{Tool, ToolConfig};
use crate::utils::{parse_frontmatter, PlanStatus};
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;

pub struct PlanListTool;

#[async_trait]
impl Tool for PlanListTool {
    fn name(&self) -> &'static str {
        "plan_list"
    }

    fn description(&self) -> &'static str {
        r#"Lists all plan files in /plans/ directory with parsed metadata.

Returns title, status, and created_at for each plan.
Optionally filter by status."#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": ["string", "null"],
                    "description": "Filter by status: draft, approved, implemented, archived"
                },
                "limit": {
                    "type": ["integer", "string", "null"],
                    "description": "Maximum number of plans to return (default: 50). Accepts integer or string."
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
        let plan_args: PlanListArgs = serde_json::from_value(args)?;

        // Parse status filter
        let status_filter = plan_args.status.and_then(|s| {
            PlanStatus::from_str(&s.to_lowercase()).ok()
        });

        let limit = plan_args.limit.unwrap_or(50);

        // Get all files in workspace root
        let all_files = file_queries::list_files_in_folder(conn, workspace_id, None).await?;

        // Filter to .plan files in /plans/ directory
        let plan_files: Vec<_> = all_files
            .into_iter()
            .filter(|f| {
                f.path.starts_with("/plans/") &&
                f.path.ends_with(".plan") &&
                matches!(f.file_type, FileType::Plan)
            })
            .collect();

        // Collect plan items with metadata
        let mut plans: Vec<PlanListItem> = Vec::new();

        for file in plan_files {
            // Extract name from path
            let name = file.path
                .strip_prefix("/plans/")
                .and_then(|s| s.strip_suffix(".plan"))
                .unwrap_or(&file.name)
                .to_string();

            // Get content and parse metadata
            let metadata = match files::get_file_with_content(conn, storage, file.id).await {
                Ok(fwc) => {
                    match &fwc.content {
                        Value::String(s) => {
                            let (meta, _) = parse_frontmatter(s);
                            meta
                        }
                        _ => None,
                    }
                }
                Err(_) => None,
            };

            // Apply status filter
            if let Some(ref filter) = status_filter {
                if metadata.as_ref().map(|m| &m.status) != Some(filter) {
                    continue;
                }
            }

            plans.push(PlanListItem {
                path: file.path,
                name,
                metadata,
            });

            if plans.len() >= limit {
                break;
            }
        }

        // Sort by created_at descending (newest first)
        plans.sort_by(|a, b| {
            let a_time = a.metadata.as_ref().map(|m| m.created_at).unwrap_or_else(chrono::Utc::now);
            let b_time = b.metadata.as_ref().map(|m| m.created_at).unwrap_or_else(chrono::Utc::now);
            b_time.cmp(&a_time)
        });

        let total = plans.len();
        let result = PlanListResult { plans, total };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
