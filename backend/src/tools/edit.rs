use crate::error::{Error, Result, ValidationErrors};
use crate::models::files::FileType;
use crate::models::requests::{
    CreateVersionRequest, ToolResponse, EditArgs, WriteResult,
};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;
use super::Tool;

/// Shared logic for edit and edit-many tools
async fn perform_edit(
    conn: &mut DbConn,
    workspace_id: Uuid,
    user_id: Uuid,
    args: EditArgs,
    require_unique: bool,
) -> Result<ToolResponse> {
    let path = super::normalize_path(&args.path);
    
    // Validation: old_string cannot be empty
    if args.old_string.is_empty() {
         return Err(Error::Validation(ValidationErrors::Single {
            field: "old_string".to_string(),
            message: "Search string cannot be empty".to_string(),
        }));
    }

    let existing_file = file_queries::get_file_by_path(conn, workspace_id, &path).await?;
    
    let file = if let Some(f) = existing_file {
        f
    } else {
        return Err(Error::NotFound(format!("File not found: {}", path)));
    };

    // Ensure it is a Document
    if !matches!(file.file_type, FileType::Document) {
         return Err(Error::Validation(ValidationErrors::Single {
            field: "path".to_string(),
            message: format!("{} tool only supports Document file types", if require_unique { "Edit" } else { "Edit-many" }),
        }));
    }

    // Get latest content
    let latest_version = file_queries::get_latest_version(conn, file.id).await?;
    
    // Optional: Reject if not read latest modification
    if let Some(last_read_hash) = args.last_read_hash {
        if latest_version.hash != last_read_hash {
            return Err(Error::Conflict(format!(
                "File content has changed since it was last read. Expected hash: {}, but latest is: {}. Please read the file again before editing.",
                last_read_hash, latest_version.hash
            )));
        }
    }

    // Extract text
    let content_text = match latest_version.content_raw.get("text") {
        Some(Value::String(s)) => s.clone(),
        _ => return Err(Error::Internal("Document content missing 'text' field".to_string())),
    };

    // Search and Count
    let matches: Vec<_> = content_text.match_indices(&args.old_string).collect();
    let count = matches.len();

    if count == 0 {
         return Err(Error::Validation(ValidationErrors::Single {
            field: "old_string".to_string(),
            message: "Search string not found in file content".to_string(),
        }));
    }

    if require_unique && count > 1 {
         return Err(Error::Validation(ValidationErrors::Single {
            field: "old_string".to_string(),
            message: format!("Search string found {} times. Please provide more context to ensure unique match.", count),
        }));
    }

    // Replace
    let new_content_text = if require_unique {
        content_text.replacen(&args.old_string, &args.new_string, 1)
    } else {
        content_text.replace(&args.old_string, &args.new_string)
    };
    
    let final_content = serde_json::json!({ "text": new_content_text });

    // Save new version
    let version = files::create_version(conn, file.id, CreateVersionRequest {
        author_id: Some(user_id),
        branch: Some("main".to_string()),
        content: final_content,
        app_data: None,
    }).await?;
    
    let result = WriteResult {
        path,
        file_id: file.id,
        version_id: version.id,
        hash: version.hash,
    };

    Ok(ToolResponse {
        success: true,
        result: serde_json::to_value(result)?,
        error: None,
    })
}

/// Edit file content tool
///
/// Edits a file by replacing a unique search string with a replacement string.
pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "edit"
    }

    fn description(&self) -> &'static str {
        "Edits a file by replacing a unique search string with a replacement string. Fails if the search string is not found or found multiple times."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(EditArgs)).unwrap_or(Value::Null)
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let edit_args: EditArgs = serde_json::from_value(args)?;
        perform_edit(conn, workspace_id, user_id, edit_args, true).await
    }
}

/// Edit many file content tool
///
/// Edits a file by replacing all occurrences of a search string with a replacement string.
pub struct EditManyTool;

#[async_trait]
impl Tool for EditManyTool {
    fn name(&self) -> &'static str {
        "edit-many"
    }

    fn description(&self) -> &'static str {
        "Edits a file by replacing all occurrences of a search string with a replacement string. Fails if the search string is not found."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(EditArgs)).unwrap_or(Value::Null)
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let edit_args: EditArgs = serde_json::from_value(args)?;
        perform_edit(conn, workspace_id, user_id, edit_args, false).await
    }
}
