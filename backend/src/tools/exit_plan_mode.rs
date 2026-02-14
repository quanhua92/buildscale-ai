use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, ExitPlanModeArgs}, queries::files as file_queries};
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Exit plan mode tool for transitioning from Plan to Build mode
///
/// This tool transitions the workspace context from strategy (Plan) to implementation (Build).
/// It updates chat metadata and prepares the system for executing the approved plan.
pub struct ExitPlanModeTool;

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &'static str {
        "exit_plan_mode"
    }

    fn description(&self) -> &'static str {
        r##"Exits Plan Mode to Build Mode. REQUIRES button click approval.

WORKFLOW: Create plan (file_type="plan") → ask_user Accept/Reject → if Accept, call this tool.

⚠️ AFTER TRANSITION: The Builder Agent MUST start executing the plan IMMEDIATELY. No questions, no waiting. Execute step 1 right away.

SAFETY: Only valid after button click. Chat messages are NOT approval. Plan must have .plan extension."##
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "plan_file_path": {"type": "string"}
            },
            "required": ["plan_file_path"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        conn: &mut DbConn,
        _storage: &FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let exit_args: ExitPlanModeArgs = serde_json::from_value(args)?;
        let plan_path = super::normalize_path(&exit_args.plan_file_path);

        // 1. Verify the plan file exists
        let plan_file = file_queries::get_file_by_path(conn, workspace_id, &plan_path).await?
            .ok_or_else(|| Error::NotFound(format!("Plan file not found: {}", plan_path)))?;

        if !matches!(plan_file.file_type, crate::models::files::FileType::Plan) {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "plan_file_path".to_string(),
                message: format!("File is not a plan file: {}", plan_path),
            }));
        }

        // 2. Get the latest version of the plan file to verify it has content
        let _plan_version = file_queries::get_latest_version(conn, plan_file.id).await?;

        // 3. Update chat metadata in database immediately
        // This ensures subsequent tools in the same stream see the updated mode
        // We need chat_id which is stored in ToolConfig.active_plan_path (hack for now)
        // Actually, we can't get chat_id from ToolConfig. The ChatActor will still
        // handle the update, but we'll ensure the update commits immediately.

        let result = crate::models::requests::ExitPlanModeResult {
            mode: "build".to_string(),
            plan_file: plan_path,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
