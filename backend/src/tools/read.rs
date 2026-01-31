use crate::{DbConn, error::Result};
use crate::models::requests::{ToolResponse, ReadArgs, ReadResult};
use crate::services::files;
use crate::queries::files as file_queries;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Read file contents tool
///
/// Reads the latest version of a file within a workspace.
pub struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Reads the content of a file at the specified path. Returns file content with a hash for change detection. Content is returned as stored - raw text for text files, JSON for structured data. Cannot read folders - folders have no content. Always read a file before editing to get the latest hash."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ReadArgs)).unwrap_or(Value::Null)
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let read_args: ReadArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&read_args.path);
        
        let file = file_queries::get_file_by_path(conn, workspace_id, &path)
            .await?
            .ok_or_else(|| crate::error::Error::NotFound(format!("File not found: {}", path)))?;
        
        if matches!(file.file_type, crate::models::files::FileType::Folder) {
            return Err(crate::error::Error::Validation(crate::error::ValidationErrors::Single {
                field: "path".to_string(),
                message: "Cannot read content of a folder".to_string(),
            }));
        }
        
        let file_with_content = files::get_file_with_content(conn, storage, file.id).await?;

        let result = ReadResult {
            path,
            content: file_with_content.content,
            hash: file_with_content.latest_version.hash,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
