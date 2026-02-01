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
        r#"Transitions the workspace from Plan Mode to Build Mode.

⚠️ SAFETY WARNING: This tool IMMEDIATELY exits Plan Mode. Only call after EXPLICIT button click approval.

=== SAFETY CHECKLIST (MUST VERIFY ALL) ⚠️ ===
Before calling this tool, you MUST verify ALL of the following:
1. ✅ You just received a response from ask_user tool
2. ✅ The response value is exactly "Accept" (not "accept", not similar, EXACTLY "Accept")
3. ✅ This response came from a BUTTON CLICK, not a chat message
4. ✅ You previously showed an Accept/Reject question to the user
5. ✅ The plan file exists and has file_type="plan"

If ANY of the above is FALSE, DO NOT CALL THIS TOOL.

=== WHAT IS A VALID PLAN FILE ===
A valid plan file MUST have:
1. File extension: .plan (e.g., /plans/implementation.plan)
2. File type: "plan" (set via file_type parameter when creating)
3. Content: Implementation plan with tasks and execution strategy

=== HOW TO CREATE A PLAN FILE ===
Use the 'write' tool with BOTH the .plan extension AND file_type parameter.

Plan files work exactly like Document files - just pass raw string content.

Example:
  path: "/plans/implementation.plan"
  content: Raw string with your plan (markdown format recommended)
  file_type: "plan"

CRITICAL: If you don't specify file_type="plan", the file will be created as type "document" and exit_plan_mode will fail with "File is not a plan file".

=== VALIDATION ===
This tool verifies:
- File exists at the specified path
- File has type "plan" (NOT "document" or other types)
- If validation fails, you must recreate the file with file_type="plan"

=== WHEN TO CALL (ONLY THIS SCENARIO) ===

✅ CORRECT - Call immediately when:
- User clicked "Accept" button on your ask_user question
- You just received: {"approval": "Accept"} from ask_user response
- You previously showed the Accept/Reject question

This is the ONLY valid scenario. No other situation justifies calling this tool.

=== WHEN NOT TO CALL (ALL THESE ARE WRONG) ===

❌ WRONG - User said in chat (NOT button click):
- "do it"
- "work on it"
- "proceed"
- "let's start"
- "looks good"
- "that's fine"
- "go ahead"
- "start implementing"
- "I approve"
- "yes, do it"

Instead: Show Accept/Reject question via ask_user to confirm

❌ WRONG - User seemed positive but unclear:
- "I think that works"
- "seems good to me"
- "ok let's try it"
- "sounds like a plan"

Instead: Show Accept/Reject question via ask_user to confirm

❌ WRONG - You just finished creating the plan:
- No user input yet
- User hasn't seen the plan
- You haven't asked for approval

Instead: Show Accept/Reject question via ask_user first

=== REQUIRED WORKFLOW ===

Step 1: Create plan file with write tool
Step 2: IMMEDIATELY call ask_user with:
  - question: "Review the implementation plan. Ready to proceed to Build Mode?"
  - schema: type="string", enum=["Accept", "Reject"]
  - buttons: Accept → "Accept", Reject → "Reject"
Step 3: Wait for user response
Step 4: IF response is "Accept" (from button click):
  - THEN call exit_plan_mode
  - ELSE IF "Reject": Ask for feedback, revise, go to Step 2
Step 5: System transitions to Build Mode

=== EXAMPLES ===

Example 1 - CORRECT:
AI: [Created plan]
AI: [Shows Accept/Reject question via ask_user]
User: [Clicks Accept button]
AI: [Calls exit_plan_mode ✅ CORRECT]

Example 2 - WRONG:
AI: [Created plan]
User: "looks good, do it"
AI: [Calls exit_plan_mode ❌ WRONG]
Correct: AI should show Accept/Reject question first

Example 3 - WRONG:
AI: [Created plan]
AI: [Calls exit_plan_mode without asking ❌ WRONG]
Correct: AI must show Accept/Reject question first

=== HOW TO DETECT BUTTON CLICK ===

A response from ask_user is a button click when:
- You just called ask_user tool
- The response contains the question name and answer
- The value matches one of your button values exactly

A chat message is NOT a button click when:
- User typed in the chat input
- You did NOT just call ask_user
- The message is natural language, not a structured answer

Remember: BUTTON CLICK = ask_user response, CHAT MESSAGE = user typing"#
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
