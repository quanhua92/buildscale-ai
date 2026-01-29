use crate::error::{Error, Result};
use crate::models::requests::{MvArgs, MvResult, ToolResponse};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;
use super::Tool;

/// Move/Rename file tool
///
/// Moves or renames a file within the workspace.
pub struct MvTool;

#[async_trait]
impl Tool for MvTool {
    fn name(&self) -> &'static str {
        "mv"
    }

    fn description(&self) -> &'static str {
        "Moves or renames a file. RENAME: provide full destination path with new filename (e.g., '/src/old.rs' -> '/src/new.rs'). MOVE: provide existing directory path (e.g., '/src/file.rs' -> '/docs/'). Destination ending with '/' is treated as directory. Fails if destination file already exists."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(MvArgs)).unwrap_or(Value::Null)
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        _storage: &FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let mv_args: MvArgs = serde_json::from_value(args)?;
        let source_path = super::normalize_path(&mv_args.source);
        let destination_path = super::normalize_path(&mv_args.destination);
        
        // 1. Resolve source file
        let source_file = file_queries::get_file_by_path(conn, workspace_id, &source_path)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Source file not found: {}", source_path)))?;
            
        // 2. Resolve destination logic
        let (target_parent_id, target_name) = if destination_path.ends_with('/') {
            // Case A: Explicit directory move "/folder/"
            let dir_path = destination_path.trim_end_matches('/');
            if dir_path.is_empty() {
                // Moving to Root
                (Some(None), source_file.name.clone())
            } else {
                let dir_file = file_queries::get_file_by_path(conn, workspace_id, dir_path)
                    .await?
                    .ok_or_else(|| Error::NotFound(format!("Destination directory not found: {}", dir_path)))?;
                    
                if !matches!(dir_file.file_type, crate::models::files::FileType::Folder) {
                    return Err(Error::Validation(crate::error::ValidationErrors::Single {
                        field: "destination".to_string(),
                        message: "Destination path ends with / but is not a directory".to_string(),
                    }));
                }
                
                (Some(Some(dir_file.id)), source_file.name.clone())
            }
        } else {
            // Case B: Check if destination exists and is a directory
            if let Some(dest_file) = file_queries::get_file_by_path(conn, workspace_id, &destination_path).await? {
                if matches!(dest_file.file_type, crate::models::files::FileType::Folder) {
                    (Some(Some(dest_file.id)), source_file.name.clone())
                } else {
                    // It's a file. This is a conflict.
                    return Err(Error::Conflict(format!(
                        "Destination file already exists: {}",
                        destination_path
                    )));
                }
            } else {
                // Case C: Rename/Move to new path
                let filename = destination_path.rsplit('/').next().unwrap_or("untitled").to_string();
                let parent_path = if let Some(idx) = destination_path.rsplit_once('/') {
                    if idx.0.is_empty() { "/" } else { idx.0 }
                } else {
                    "/"
                };
                
                let parent_id = if parent_path == "/" {
                    Some(None)
                } else {
                    let p = file_queries::get_file_by_path(conn, workspace_id, parent_path)
                        .await?
                        .ok_or_else(|| Error::NotFound(format!("Destination parent directory not found: {}", parent_path)))?;
                    Some(Some(p.id))
                };
                
                (parent_id, filename)
            }
        };
        
        // 3. Safety check: prevent moving a folder into itself or a subfolder
        if source_file.file_type == crate::models::files::FileType::Folder {
            if let Some(Some(parent_id)) = target_parent_id {
                if file_queries::is_descendant_of(conn, parent_id, source_file.id).await? {
                    return Err(Error::Validation(crate::error::ValidationErrors::Single {
                        field: "destination".to_string(),
                        message: "Cannot move a folder into itself or a subfolder.".to_string(),
                    }));
                }
            }
        }
        
        let update_request = crate::models::requests::UpdateFileRequest {
            parent_id: target_parent_id,
            name: Some(target_name),
            slug: None,
            is_virtual: None,
            is_remote: None,
            permission: None,
        };
        
        let updated_file = files::update_file(conn, source_file.id, update_request).await?;
        
        let result = MvResult {
            from_path: source_path,
            to_path: updated_file.path,
        };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
