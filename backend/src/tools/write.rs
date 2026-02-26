use crate::error::{Error, Result, ValidationErrors};
use crate::models::files::FileType;
use crate::models::requests::{
    CreateFileRequest, ToolResponse, WriteArgs, WriteResult,
};
use crate::queries::files as file_queries;
use crate::services::files;
use crate::state::TagIndexMessage;
use crate::utils::{DocumentMetadata, prepend_yaml_frontmatter};
use crate::DbConn;
use async_trait::async_trait;
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;
use super::{Tool, ToolConfig};

/// Write file contents tool
///
/// Creates a new file or updates an existing file with new content.
pub struct WriteTool;

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "write"
    }

    fn description(&self) -> &'static str {
        "Creates a new file or completely replaces existing file content. Content is stored as-is: strings are stored as raw text, JSON objects are stored as structured data. CRITICAL: This is NOT for partial edits - use 'edit' tool to modify specific sections. Use 'write' only for new files or complete file replacement. OVERWRITE PROTECTION: By default (overwrite=false), returns error if file exists to prevent accidental overwrites. Set overwrite=true to explicitly replace existing files. For modifying existing files, 'edit' tool is recommended."
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "content": {
                    "type": "string",
                    "description": "File content as a string (for JSON content, pass as serialized JSON string)"
                },
                "file_type": {"type": ["string", "null"]},
                "overwrite": {
                    "type": ["boolean", "string"],
                    "description": "Accepts JSON boolean (true/false) or string representations ('true', 'True', 'false', 'False', 'TRUE', 'FALSE'). If false (default), returns error when file exists to prevent accidental overwrites. Set to true to explicitly overwrite existing files. Recommendation: Use 'edit' tool for modifying existing files instead of overwriting."
                }
            },
            "required": ["path", "content"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &crate::services::storage::FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let write_args: WriteArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&write_args.path);

        let existing_file = file_queries::get_file_by_path(conn, workspace_id, &path).await?;

        // Overwrite Protection
        if existing_file.is_some() && !write_args.overwrite {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: format!(
                    "File already exists: {}. To overwrite, set overwrite=true. \
                    However, for modifying existing files, the 'edit' tool is recommended instead of overwriting.",
                    path
                ),
            }));
        }

        // Plan Mode Guard
        if config.plan_mode {
            let is_plan_file = if let Some(ref file) = existing_file {
                matches!(file.file_type, FileType::Plan)
            } else {
                path.ends_with(".plan")
            };

            if !is_plan_file {
                return Err(Error::Validation(ValidationErrors::Single {
                    field: "path".to_string(),
                    message: super::PLAN_MODE_ERROR.to_string(),
                }));
            }
        }

        let (result, is_markdown_document) = if let Some(file) = existing_file {
            // Update existing file
            let final_content = Self::prepare_content_for_update(file.file_type, write_args.content.0)?;

            let updated_file = files::update_file_content(conn, storage, file.id, final_content).await?;

            let is_markdown = matches!(file.file_type, FileType::Document) && path.ends_with(".md");
            (
                WriteResult {
                    path,
                    file_id: file.id,
                    hash: updated_file.hash.unwrap_or_default(),
                },
                is_markdown,
            )
        } else {
            // Create new file
            let filename = path.rsplit('/').next().unwrap_or("untitled");

            let file_type = if let Some(ft_str) = write_args.file_type.as_deref() {
                FileType::from_str(ft_str).map_err(|_| {
                    Error::Validation(ValidationErrors::Single {
                        field: "file_type".to_string(),
                        message: format!("Invalid file type: {}", ft_str),
                    })
                })?
            } else {
                FileType::Document
            };

            let final_content = Self::prepare_content_for_create(file_type, filename, write_args.content.0, write_args.file_type.as_deref())?;

            let file_result = files::create_file_with_content(conn, storage, CreateFileRequest {
                workspace_id,
                parent_id: None,
                name: filename.to_string(),
                path: Some(path.clone()),
                file_type,
                content: final_content,
            }).await?;

            let is_markdown = matches!(file_type, FileType::Document) && path.ends_with(".md");
            (
                WriteResult {
                    path,
                    file_id: file_result.file.id,
                    hash: file_result.hash,
                },
                is_markdown,
            )
        };

        // Signal tag indexer to update tags for markdown documents
        if is_markdown_document {
            if let Some(ref tag_index_tx) = config.tag_index_tx {
                if let Err(e) = tag_index_tx.send(TagIndexMessage {
                    workspace_id,
                    file_id: result.file_id,
                }) {
                    tracing::warn!("Failed to signal tag indexer: {}", e);
                }
            }
        }

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

impl WriteTool {
    /// Prepare content for creating a new file (adds frontmatter for markdown documents)
    fn prepare_content_for_create(
        file_type: FileType,
        filename: &str,
        content: Value,
        requested_type_str: Option<&str>,
    ) -> Result<Value> {
        // Only block folder writes if not explicitly requested
        if matches!(file_type, FileType::Folder) && requested_type_str != Some("folder") {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: "Cannot write text content to a folder path".to_string(),
            }));
        }

        // For markdown documents, auto-add frontmatter if not present (Obsidian-style)
        if matches!(file_type, FileType::Document) && filename.ends_with(".md") {
            let content_str = match &content {
                Value::String(s) => s.clone(),
                _ => serde_json::to_string(&content).unwrap_or_default(),
            };

            // Check if content already has frontmatter
            if !content_str.trim_start().starts_with("---\n") {
                // Auto-generate frontmatter from filename
                let metadata = DocumentMetadata::from_filename(filename);
                let content_with_frontmatter = prepend_yaml_frontmatter(&metadata, &content_str);
                return Ok(serde_json::json!(content_with_frontmatter));
            }
        }

        Ok(content)
    }

    /// Prepare content for updating an existing file (updates modified timestamp for markdown documents)
    fn prepare_content_for_update(
        file_type: FileType,
        content: Value,
    ) -> Result<Value> {
        // Only block folder writes
        if matches!(file_type, FileType::Folder) {
            return Err(Error::Validation(ValidationErrors::Single {
                field: "path".to_string(),
                message: "Cannot write text content to a folder path".to_string(),
            }));
        }

        // For markdown documents, update modified timestamp if frontmatter exists
        if matches!(file_type, FileType::Document) {
            let content_str = match &content {
                Value::String(s) => s.clone(),
                _ => serde_json::to_string(&content).unwrap_or_default(),
            };

            use crate::utils::parse_yaml_frontmatter;
            let (metadata, body) = parse_yaml_frontmatter::<DocumentMetadata>(&content_str);

            if let Some(mut meta) = metadata {
                // Update modified timestamp
                meta.touch();
                let content_with_frontmatter = prepend_yaml_frontmatter(&meta, body);
                return Ok(serde_json::json!(content_with_frontmatter));
            }
        }

        Ok(content)
    }
}
