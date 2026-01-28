use crate::error::{Error, Result, ValidationErrors};
use crate::models::files::FileType;
use crate::models::requests::{
    CreateVersionRequest, ToolResponse, EditArgs, WriteResult,
};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;
use super::Tool;

/// Helper to get file content with disk fallback
async fn get_file_content_for_edit(
    conn: &mut DbConn,
    storage: &FileStorageService,
    file_id: Uuid,
) -> Result<serde_json::Value> {
    let file_with_content = files::get_file_with_content(conn, storage, file_id).await?;
    Ok(file_with_content.latest_version.content_raw)
}

/// Shared logic for edit tool
async fn perform_edit(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    user_id: Uuid,
    args: EditArgs,
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

    // Virtual File Protection: Prevent direct edits to system-managed files (e.g. Chats)
    if file.is_virtual {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "path".to_string(),
            message: "Cannot edit a virtual file directly. Use specialized system tools (e.g., chat API) to modify this resource.".to_string(),
        }));
    }

    // Folders cannot be edited as text
    if matches!(file.file_type, FileType::Folder) {
         return Err(Error::Validation(ValidationErrors::Single {
            field: "path".to_string(),
            message: "Cannot edit a folder. Edit tool only works on files with text content.".to_string(),
        }));
    }

    // Get latest content (with disk fallback)
    let content_raw = get_file_content_for_edit(conn, storage, file.id).await?;

    // Get the version hash for validation
    let latest_version = file_queries::get_latest_version(conn, file.id).await?;

    // Optional: Reject if not read latest modification
    if let Some(last_read_hash) = args.last_read_hash
        && latest_version.hash != last_read_hash
    {
        return Err(Error::Conflict(format!(
            "File content has changed since it was last read. Expected hash: {}, but latest is: {}. Please read the file again before editing.",
            last_read_hash, latest_version.hash
        )));
    }

    // Extract text representation for editing
    let content_text = match content_raw.get("text") {
        Some(Value::String(s)) => s.clone(),
        _ => {
            if let Some(s) = content_raw.as_str() {
                s.to_string()
            } else {
                // For non-standard types, try recursive extraction
                let extracted = files::extract_text_recursively(&content_raw);
                if extracted.is_empty() {
                    return Err(Error::Validation(ValidationErrors::Single {
                        field: "path".to_string(),
                        message: "File content does not contain editable text".to_string(),
                    }));
                }
                extracted
            }
        },
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

    if count > 1 {
         return Err(Error::Validation(ValidationErrors::Single {
            field: "old_string".to_string(),
            message: format!("Search string found {} times. Please provide more context to ensure unique match.", count),
        }));
    }

    // Replace
    let new_content_text = content_text.replacen(&args.old_string, &args.new_string, 1);

    // Store as raw string (not wrapped in {"text": ...})
    let final_content = serde_json::json!(new_content_text);

    // Save new version
    let version = files::create_version(conn, storage, file.id, CreateVersionRequest {
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
        "Edits a file by replacing a unique search string with a replacement string. CRITICAL: (1) old_string MUST be non-empty and unique in file. (2) This is a REPLACE operation - old_string is completely removed and replaced by new_string. (3) To preserve original content, you MUST include it in new_string. (4) Always use last_read_hash from prior read to prevent conflicts. Fails if old_string is empty, not found, or found multiple times."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(EditArgs)).unwrap_or(Value::Null)
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let edit_args: EditArgs = serde_json::from_value(args)?;
        perform_edit(conn, storage, workspace_id, user_id, edit_args).await
    }
}
