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
        
        let path_pattern = grep_args.path_pattern.map(|mut p| {
            // Convert glob-like * to SQL LIKE %
            p = p.replace('*', "%");
            
            // If it doesn't look like an absolute path or already a wildcard, make it fuzzy
            if !p.starts_with('/') && !p.starts_with('%') {
                p.insert(0, '%');
            }
            
            // If it's a directory path, match children
            if p.ends_with('/') && !p.ends_with('%') {
                p.push('%');
            }
            p
        });

        let matches = file_queries::grep_files(
            conn, 
            workspace_id, 
            &grep_args.pattern, 
            path_pattern.as_deref(), 
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
