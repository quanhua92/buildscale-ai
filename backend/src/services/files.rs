use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::{
        files::{File, FileStatus, FileType, NewFile, NewFileVersion},
        requests::{CreateFileRequest, CreateVersionRequest, FileNetworkSummary, FileWithContent, UpdateFileHttp},
    },
    queries::files,
    validation::validate_file_slug,
};
use sha2::{Digest, Sha256};
use sqlx::Acquire;
use uuid::Uuid;

/// Hashes JSON content using SHA-256 for content-addressing
pub fn hash_content(content: &serde_json::Value) -> String {
    let content_str = content.to_string();
    let mut hasher = Sha256::new();
    hasher.update(content_str.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Creates a new file with its initial content version in a single transaction
pub async fn create_file_with_content(
    conn: &mut DbConn,
    request: CreateFileRequest,
) -> Result<FileWithContent> {
    // 1. Validate inputs
    validate_file_slug(&request.slug)?;

    // 2. Start transaction
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // 3. Create file identity record
    let new_file = NewFile {
        workspace_id: request.workspace_id,
        parent_id: request.parent_id,
        author_id: request.author_id,
        file_type: request.file_type,
        status: FileStatus::Ready, // Set to Ready since we are providing content immediately
        slug: request.slug,
    };
    let file = files::create_file_identity(&mut tx, new_file).await?;

    // 4. Calculate content hash
    let hash = hash_content(&request.content);

    // 5. Create first version record
    let new_version = NewFileVersion {
        file_id: file.id,
        branch: "main".to_string(),
        content_raw: request.content,
        app_data: request.app_data.unwrap_or(serde_json::json!({})),
        hash,
        author_id: Some(request.author_id),
    };
    let latest_version = files::create_version(&mut tx, new_version).await?;

    // 6. Commit transaction
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(FileWithContent {
        file,
        latest_version,
    })
}

/// Creates a new version for an existing file
///
/// This method implements deduplication: if the content hash matches the latest
/// version, it skips the database insert and returns the existing version.
pub async fn create_version(
    conn: &mut DbConn,
    file_id: Uuid,
    request: CreateVersionRequest,
) -> Result<crate::models::files::FileVersion> {
    let hash = hash_content(&request.content);

    // 1. Check if the latest version already has this hash (deduplication)
    let latest = files::get_latest_version_optional(conn, file_id).await?;
    if let Some(v) = latest.filter(|v| v.hash == hash) {
        return Ok(v);
    }

    // 2. Insert new version
    let new_version = NewFileVersion {
        file_id,
        branch: request.branch.unwrap_or_else(|| "main".to_string()),
        content_raw: request.content,
        app_data: request.app_data.unwrap_or(serde_json::json!({})),
        hash,
        author_id: request.author_id,
    };

    files::create_version(conn, new_version).await
}

/// Gets a file and its latest version together
pub async fn get_file_with_content(conn: &mut DbConn, file_id: Uuid) -> Result<FileWithContent> {
    let file = files::get_file_by_id(conn, file_id).await?;
    let latest_version = files::get_latest_version(conn, file_id).await?;

    Ok(FileWithContent {
        file,
        latest_version,
    })
}

/// Updates a file's metadata (move and/or rename)
pub async fn move_or_rename_file(
    conn: &mut DbConn,
    file_id: Uuid,
    request: UpdateFileHttp,
) -> Result<File> {
    // 1. Get current file state
    let current_file = files::get_file_by_id(conn, file_id).await?;

    // 2. Determine target values
    let target_parent_id = request.parent_id.or(current_file.parent_id);
    let target_slug = request.slug.as_deref().unwrap_or(&current_file.slug);

    // 3. Validation
    if let Some(new_slug) = &request.slug {
        validate_file_slug(new_slug)?;
    }

    // 4. Check if anything actually changed
    if target_parent_id == current_file.parent_id && target_slug == current_file.slug {
        return Ok(current_file);
    }

    // 5. Start transaction
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // 6. Cycle Detection (if moving)
    if request.parent_id.is_some() {
        if let Some(new_parent_id) = request.parent_id {
            // Cannot move to itself
            if new_parent_id == file_id {
                return Err(Error::Validation(crate::error::ValidationErrors::Single {
                    field: "parent_id".to_string(),
                    message: "Cannot move a folder into itself".to_string(),
                }));
            }

            // Cannot move to a descendant
            if files::is_descendant_of(&mut tx, new_parent_id, file_id).await? {
                return Err(Error::Validation(crate::error::ValidationErrors::Single {
                    field: "parent_id".to_string(),
                    message: "Cannot move a folder into one of its own subfolders".to_string(),
                }));
            }
        }
    }

    // 7. Collision Check
    if files::check_slug_collision(&mut tx, current_file.workspace_id, target_parent_id, target_slug).await? {
        return Err(Error::Conflict(format!(
            "A file with name '{}' already exists in the target folder",
            target_slug
        )));
    }

    // 8. Update metadata
    let updated_file = files::update_file_metadata(&mut tx, file_id, target_parent_id, target_slug).await?;

    // 9. Commit
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(updated_file)
}

/// Soft deletes a file with a check for empty folders
pub async fn soft_delete_file(conn: &mut DbConn, file_id: Uuid) -> Result<()> {
    let file = files::get_file_by_id(conn, file_id).await?;

    // If it's a folder, ensure it's empty
    if matches!(file.file_type, FileType::Folder) {
        if files::has_active_children(conn, file_id).await? {
            return Err(Error::Conflict(
                "Cannot delete folder because it is not empty. Please delete or move sub-items first.".to_string(),
            ));
        }
    }

    files::soft_delete_file(conn, file_id).await
}

/// Restores a soft-deleted file
pub async fn restore_file(conn: &mut DbConn, file_id: Uuid) -> Result<File> {
    let file = files::get_file_by_id(conn, file_id).await?;

    if file.deleted_at.is_none() {
        return Ok(file);
    }

    // Collision check: can it return to its original home?
    if files::check_slug_collision(conn, file.workspace_id, file.parent_id, &file.slug).await? {
        return Err(Error::Conflict(format!(
            "Cannot restore '{}' because another file with the same name already exists in its original location.",
            file.slug
        )));
    }

    files::restore_file(conn, file_id).await
}

/// Lists all items in the trash for a workspace
pub async fn list_trash(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<File>> {
    files::list_trash(conn, workspace_id).await
}

// ============================================================================
// TAGGING SERVICES
// ============================================================================

/// Adds a tag to a file
pub async fn add_tag(conn: &mut DbConn, file_id: Uuid, tag: &str) -> Result<()> {
    let tag = tag.trim().to_lowercase();

    if tag.is_empty() {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "tag".to_string(),
            message: "Tag cannot be empty".to_string(),
        }));
    }

    if tag.len() > 50 {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "tag".to_string(),
            message: "Tag must be 50 characters or less".to_string(),
        }));
    }

    files::add_tag(conn, file_id, &tag).await
}

/// Removes a tag from a file
pub async fn remove_tag(conn: &mut DbConn, file_id: Uuid, tag: &str) -> Result<()> {
    files::remove_tag(conn, file_id, tag).await
}

/// Lists files by tag in a workspace
pub async fn list_files_by_tag(
    conn: &mut DbConn,
    workspace_id: Uuid,
    tag: &str,
) -> Result<Vec<File>> {
    files::list_files_by_tag(conn, workspace_id, tag).await
}

// ============================================================================
// LINKING SERVICES
// ============================================================================

/// Creates a link between two files
pub async fn link_files(conn: &mut DbConn, source_id: Uuid, target_id: Uuid) -> Result<()> {
    if source_id == target_id {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "target_file_id".to_string(),
            message: "A file cannot link to itself".to_string(),
        }));
    }

    // Boundary Check: Do they belong to the same workspace?
    let source = files::get_file_by_id(conn, source_id).await?;
    let target = files::get_file_by_id(conn, target_id).await?;

    if source.workspace_id != target.workspace_id {
        return Err(Error::Forbidden(
            "Cannot link files across different workspaces".to_string(),
        ));
    }

    files::add_link(conn, source_id, target_id).await
}

/// Removes a link between two files
pub async fn remove_link(conn: &mut DbConn, source_id: Uuid, target_id: Uuid) -> Result<()> {
    files::remove_link(conn, source_id, target_id).await
}

/// Gets the local network summary for a file
pub async fn get_file_network(conn: &mut DbConn, file_id: Uuid) -> Result<FileNetworkSummary> {
    let tags = files::get_tags_for_file(conn, file_id).await?;
    let outbound_links = files::get_outbound_links(conn, file_id).await?;
    let backlinks = files::get_backlinks(conn, file_id).await?;

    Ok(FileNetworkSummary {
        tags,
        outbound_links,
        backlinks,
    })
}
