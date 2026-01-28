use crate::{DbConn, error::Result, models::requests::{ToolResponse, TouchArgs, TouchResult}, services::files, services::storage::FileStorageService, queries::files as file_queries};
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::Tool;

/// Update file timestamp or create empty file
pub struct TouchTool;

#[async_trait]
impl Tool for TouchTool {
    fn name(&self) -> &'static str {
        "touch"
    }

    fn description(&self) -> &'static str {
        "Updates file timestamp if it exists, or creates an empty Document file if it doesn't. Created files have empty text content. Use this to create placeholder files or refresh file timestamps. Does not create directories - use 'mkdir' instead."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(TouchArgs)).unwrap_or(Value::Null)
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let touch_args: TouchArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&touch_args.path);
        
        // Check if file exists
        let existing_file = file_queries::get_file_by_path(conn, workspace_id, &path).await?;
        
        let file_id = if let Some(file) = existing_file {
            // Update timestamp
            file_queries::touch_file(conn, file.id).await?;
            file.id
        } else {
            // Create empty file
            let filename = path.rsplit('/').next().unwrap_or("untitled");
            let file_type = crate::models::files::FileType::Document; 
            
            let req = crate::models::requests::CreateFileRequest {
                workspace_id,
                parent_id: None,
                author_id: user_id,
                name: filename.to_string(),
                slug: None,
                path: Some(path.clone()),
                is_virtual: None,
                is_remote: None,
                permission: None,
                file_type,
                content: serde_json::json!(""), 
                app_data: None,
            };
            
            let file_with_content = files::create_file_with_content(conn, storage, req).await?;
            file_with_content.file.id
        };
        
        let result = TouchResult {
            path: path.clone(),
            file_id,
        };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
