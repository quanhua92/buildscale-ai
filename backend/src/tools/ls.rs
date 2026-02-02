use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, LsArgs, LsResult, LsEntry}, queries::files};
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// List directory contents tool
///
/// Lists files and folders in a directory within a workspace.
pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &'static str {
        "ls"
    }

    fn description(&self) -> &'static str {
        "Lists files and folders in a workspace directory. All parameters are optional. Returns entries sorted with folders first.

Parameters:
- path (string, optional): workspace directory path. Default: '/' for workspace root.
- recursive (boolean, optional): list all subdirectories recursively. Default: false.

USAGE EXAMPLES:
- Good (list root): {}
- Good (list specific folder): {\"path\": \"/src\"}
- Good (recursive listing): {\"path\": \"/src\", \"recursive\": true}
- Good (explicit nulls): {\"path\": null, \"recursive\": null}

BAD EXAMPLES (will fail):
- Bad (string instead of object): \"/src\"
- Bad (array instead of object): [\"/src\"]
- Bad (extra properties): {\"path\": \"/src\", \"invalid\": true}"
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": ["string", "null"]},
                "recursive": {"type": ["boolean", "null"]}
            },
            "additionalProperties": false
        })
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        _storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let ls_args: LsArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&ls_args.path.unwrap_or_else(|| "/".to_string()));
        let recursive = ls_args.recursive.unwrap_or(false);
        
        let parent_id = if path == "/" {
            None
        } else {
            let parent_file = files::get_file_by_path(conn, workspace_id, &path)
                .await?
                .ok_or_else(|| Error::NotFound(format!("Directory not found: {}", path)))?;
            
            if !matches!(parent_file.file_type, crate::models::files::FileType::Folder) {
                return Err(Error::Validation(crate::error::ValidationErrors::Single {
                    field: "path".to_string(),
                    message: format!("Path is not a directory: {}", path),
                }));
            }
            
            Some(parent_file.id)
        };
        
        let files = if recursive {
            Self::list_files_recursive(conn, workspace_id, &path).await?
        } else {
            files::list_files_in_folder(conn, workspace_id, parent_id).await?
        };
        
        let entries: Vec<LsEntry> = files.into_iter().map(|f| LsEntry {
            id: f.id,
            name: f.slug,
            display_name: f.name,
            path: f.path,
            file_type: f.file_type,
            is_virtual: f.is_virtual,
            updated_at: f.updated_at,
        }).collect();
        
        let result = LsResult { path, entries };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

impl LsTool {
    async fn list_files_recursive(
        conn: &mut DbConn,
        workspace_id: Uuid,
        path_prefix: &str,
    ) -> Result<Vec<crate::models::files::File>> {
        let files = sqlx::query_as!(
            crate::models::files::File,
            r#"
            SELECT
                id, workspace_id, parent_id, author_id,
                file_type as "file_type: crate::models::files::FileType",
                status as "status: crate::models::files::FileStatus",
                name, slug, path,
                is_virtual, is_remote, permission,
                latest_version_id,
                deleted_at, created_at, updated_at
            FROM files
            WHERE workspace_id = $1
              AND path LIKE $2 || '%'
              AND path != $2
              AND deleted_at IS NULL
            ORDER BY path ASC
            "#,
            workspace_id,
            path_prefix
        )
        .fetch_all(conn)
        .await
        .map_err(Error::Sqlx)?;

        Ok(files)
    }
}
