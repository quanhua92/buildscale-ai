use crate::{DbConn, error::Result};
use crate::models::requests::{ToolResponse, MkdirArgs, MkdirResult};
use crate::services::files as file_services;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::Tool;

/// Mkdir tool for creating directories
///
/// Recursively creates folders to ensure a path exists.
pub struct MkdirTool;

#[async_trait]
impl Tool for MkdirTool {
    fn name(&self) -> &'static str {
        "mkdir"
    }

    fn description(&self) -> &'static str {
        "Recursively creates directories to ensure the full path exists. Creates all parent directories automatically if they don't exist. For example, 'mkdir /a/b/c' will create /a, /a/b, and /a/b/c as needed. Succeeds silently if path already exists."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(MkdirArgs)).unwrap_or(Value::Null)
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let mkdir_args: MkdirArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&mkdir_args.path);
        
        let folder_id = file_services::ensure_path_exists(
            conn, 
            workspace_id, 
            &path, 
            user_id
        ).await?;
        
        let result = MkdirResult {
            path,
            file_id: folder_id,
        };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
