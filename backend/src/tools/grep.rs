use crate::{DbConn, error::Result};
use crate::models::requests::{ToolResponse, GrepArgs, GrepResult};
use crate::queries::files as file_queries;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::Tool;

/// Grep tool for searching file contents using regex
///
/// Searches for a regex pattern across all document files in a workspace.
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Searches for a regex pattern in all document files within the workspace. Supports optional path filtering."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GrepArgs)).unwrap_or(Value::Null)
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        _user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let grep_args: GrepArgs = serde_json::from_value(args)?;
        
        let matches = file_queries::grep_files(
            conn, 
            workspace_id, 
            &grep_args.pattern, 
            grep_args.path_pattern.as_deref(), 
            grep_args.case_sensitive.unwrap_or(false)
        ).await?;
        
        let result = GrepResult {
            matches,
        };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
