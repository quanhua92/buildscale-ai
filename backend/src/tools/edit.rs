use crate::error::{Error, Result, ValidationErrors};
use crate::models::files::FileType;
use crate::models::requests::{
    ToolResponse, EditArgs, WriteResult,
};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::services::storage::FileStorageService;
use crate::tools::helpers;
use crate::utils::{parse_yaml_frontmatter, prepend_yaml_frontmatter, DocumentMetadata};
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;
use super::{Tool, ToolConfig};

/// Helper to get file content with disk fallback
async fn get_file_content_for_edit(
    conn: &mut DbConn,
    storage: &FileStorageService,
    file_id: Uuid,
) -> Result<serde_json::Value> {
    let file_with_content = files::get_file_with_content(conn, storage, file_id).await?;
    Ok(file_with_content.content)
}

/// Shared logic for edit tool
async fn perform_edit(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    _user_id: Uuid,
    config: ToolConfig,
    args: EditArgs,
) -> Result<ToolResponse> {
    let path = super::normalize_path(&args.path);

    // Determine operation type
    let is_replace = args.old_string.is_some() && args.new_string.is_some();
    let is_insert = args.insert_line.is_some() && args.insert_content.is_some();

    // Validation
    if !is_replace && !is_insert {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "operation".to_string(),
            message: "Must specify either (old_string + new_string) for Replace operation or (insert_line + insert_content) for Insert operation".to_string(),
        }));
    }

    if is_replace && is_insert {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "operation".to_string(),
            message: "Cannot specify both Replace and Insert operations. Choose one.".to_string(),
        }));
    }

    if is_insert {
        return perform_insert(conn, storage, workspace_id, config, path, args).await;
    }

    perform_replace(conn, storage, workspace_id, config, path, args).await
}

/// Perform Insert operation
async fn perform_insert(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    config: ToolConfig,
    path: String,
    args: EditArgs,
) -> Result<ToolResponse> {
    let insert_line = args.insert_line.unwrap();
    let insert_content = args.insert_content.unwrap();

    if insert_content.is_empty() {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "insert_content".to_string(),
            message: "Insert content cannot be empty".to_string(),
        }));
    }

    let existing_file = file_queries::get_file_by_path(conn, workspace_id, &path).await?;

    let file = if let Some(f) = existing_file {
        f
    } else {
        match helpers::file_exists_on_disk(storage, workspace_id, &path).await {
            Ok(true) => {
                helpers::import_file_to_database(conn, storage, workspace_id, &path, Uuid::nil()).await?
            }
            Ok(false) => {
                return Err(Error::NotFound(format!("File not found: {}", path)));
            }
            Err(e) => {
                return Err(e);
            }
        }
    };

    // Plan Mode Guard
    if config.plan_mode && !matches!(file.file_type, FileType::Plan) {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "path".to_string(),
            message: super::PLAN_MODE_ERROR.to_string(),
        }));
    }

    // Folders cannot be edited as text
    if matches!(file.file_type, FileType::Folder) {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "path".to_string(),
            message: "Cannot edit a folder. Edit tool only works on files with text content.".to_string(),
        }));
    }

    // Get current content
    let file_content = get_file_content_for_edit(conn, storage, file.id).await?;

    // Hash validation
    if let Some(last_read_hash) = args.last_read_hash {
        let current_hash = file.hash.clone().unwrap_or_default();
        if current_hash != last_read_hash {
            return Err(Error::Conflict(format!(
                "File content has changed since it was last read. Expected hash: {}, but latest is: {}. Please read the file again before editing.",
                last_read_hash, current_hash
            )));
        }
    }

    // Extract text
    let content_text = match file_content.get("text") {
        Some(Value::String(s)) => s.clone(),
        _ => {
            if let Some(s) = file_content.as_str() {
                s.to_string()
            } else {
                let extracted = files::extract_text_recursively(&file_content);
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

    // For markdown documents, parse frontmatter and work on body content
    let (metadata, body_content) = if matches!(file.file_type, FileType::Document) && path.ends_with(".md") {
        let (meta, body) = parse_yaml_frontmatter::<DocumentMetadata>(&content_text);
        (meta, body.to_string())
    } else {
        (None, content_text.clone())
    };

    let mut lines: Vec<&str> = body_content.lines().collect();

    if insert_line > lines.len() {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "insert_line".to_string(),
            message: format!("Insert line {} is out of bounds (file has {} lines)", insert_line, lines.len()),
        }));
    }

    lines.insert(insert_line, &insert_content);
    let new_body = lines.join("\n");

    // Re-add frontmatter for documents
    let final_content = if let Some(mut meta) = metadata {
        meta.touch();
        serde_json::json!(prepend_yaml_frontmatter(&meta, &new_body))
    } else {
        serde_json::json!(new_body)
    };

    // Update file
    let updated_file = files::update_file_content(conn, storage, file.id, final_content).await?;

    let result = WriteResult {
        path,
        file_id: file.id,
        hash: updated_file.hash.unwrap_or_default(),
    };

    Ok(ToolResponse {
        success: true,
        result: serde_json::to_value(result)?,
        error: None,
    })
}

/// Perform Replace operation
async fn perform_replace(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    config: ToolConfig,
    path: String,
    args: EditArgs,
) -> Result<ToolResponse> {
    let old_string = args.old_string.unwrap();
    let new_string = args.new_string.unwrap();

    if old_string.is_empty() {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "old_string".to_string(),
            message: "Search string cannot be empty".to_string(),
        }));
    }

    let existing_file = file_queries::get_file_by_path(conn, workspace_id, &path).await?;

    let file = if let Some(f) = existing_file {
        f
    } else {
        match helpers::file_exists_on_disk(storage, workspace_id, &path).await {
            Ok(true) => {
                helpers::import_file_to_database(conn, storage, workspace_id, &path, Uuid::nil()).await?
            }
            Ok(false) => {
                return Err(Error::NotFound(format!("File not found: {}", path)));
            }
            Err(e) => {
                return Err(e);
            }
        }
    };

    // Plan Mode Guard
    if config.plan_mode && !matches!(file.file_type, FileType::Plan) {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "path".to_string(),
            message: super::PLAN_MODE_ERROR.to_string(),
        }));
    }

    // Folders cannot be edited
    if matches!(file.file_type, FileType::Folder) {
        return Err(Error::Validation(ValidationErrors::Single {
            field: "path".to_string(),
            message: "Cannot edit a folder. Edit tool only works on files with text content.".to_string(),
        }));
    }

    // Get current content
    let file_content = get_file_content_for_edit(conn, storage, file.id).await?;

    // Hash validation
    if let Some(last_read_hash) = args.last_read_hash {
        let current_hash = file.hash.clone().unwrap_or_default();
        if current_hash != last_read_hash {
            return Err(Error::Conflict(format!(
                "File content has changed since it was last read. Expected hash: {}, but latest is: {}. Please read the file again before editing.",
                last_read_hash, current_hash
            )));
        }
    }

    // Extract text
    let content_text = match file_content.get("text") {
        Some(Value::String(s)) => s.clone(),
        _ => {
            if let Some(s) = file_content.as_str() {
                s.to_string()
            } else {
                let extracted = files::extract_text_recursively(&file_content);
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

    // For markdown documents, parse frontmatter and work on body content
    let (metadata, body_content) = if matches!(file.file_type, FileType::Document) && path.ends_with(".md") {
        let (meta, body) = parse_yaml_frontmatter::<DocumentMetadata>(&content_text);
        (meta, body.to_string())
    } else {
        (None, content_text.clone())
    };

    // Search and validate
    let matches: Vec<_> = body_content.match_indices(&old_string).collect();
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
    let new_body = body_content.replacen(&old_string, &new_string, 1);

    // Re-add frontmatter for documents
    let final_content = if let Some(mut meta) = metadata {
        meta.touch();
        serde_json::json!(prepend_yaml_frontmatter(&meta, &new_body))
    } else {
        serde_json::json!(new_body)
    };

    // Update file
    let updated_file = files::update_file_content(conn, storage, file.id, final_content).await?;

    let result = WriteResult {
        path,
        file_id: file.id,
        hash: updated_file.hash.unwrap_or_default(),
    };

    Ok(ToolResponse {
        success: true,
        result: serde_json::to_value(result)?,
        error: None,
    })
}

/// Edit file content tool
pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "edit"
    }

    fn description(&self) -> &'static str {
        r#"Edits files via REPLACE or INSERT. Read file first for last_read_hash.

REPLACE: {"path":"/f","old_string":"exact","new_string":"new","last_read_hash":"x"} - old_string must be unique.
INSERT: {"path":"/f","insert_line":5,"insert_content":"line","last_read_hash":"x"} - line is 0-indexed."#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "old_string": {"type": ["string", "null"], "description": "For REPLACE: text to find and replace (must be unique and non-empty)"},
                "new_string": {"type": ["string", "null"], "description": "For REPLACE: replacement text"},
                "insert_line": {"type": ["integer", "string", "null"], "description": "For INSERT: line number (0-indexed) where content is added. Accepts integer or string (e.g., 0 or '0')."},
                "insert_content": {"type": ["string", "null"], "description": "For INSERT: content to insert at insert_line"},
                "last_read_hash": {"type": ["string", "null"], "description": "Hash from latest read (prevents conflicts)"}
            },
            "required": ["path"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        user_id: Uuid,
        config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let edit_args: EditArgs = serde_json::from_value(args)?;
        perform_edit(conn, storage, workspace_id, user_id, config, edit_args).await
    }
}
