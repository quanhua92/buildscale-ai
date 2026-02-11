//! Filesystem helper functions for tools
//!
//! This module provides helper functions for working with files on disk,
//! particularly for files that may not be in the database (e.g., files created
//! via SSH, migration scripts, or external tools).

use crate::{DbConn, error::Result, models::files::FileType};
use crate::models::requests::CreateFileRequest;
use crate::services::storage::FileStorageService;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use std::path::Path;

/// Check if a file exists on disk
///
/// # Arguments
/// * `storage` - The file storage service
/// * `workspace_id` - The workspace ID
/// * `path` - The file path (with or without leading slash)
///
/// # Returns
/// * `Ok(true)` if the file exists on disk
/// * `Ok(false)` if the file does not exist on disk
/// * `Err(_)` if there was an error checking
pub async fn file_exists_on_disk(
    storage: &FileStorageService,
    workspace_id: Uuid,
    path: &str,
) -> Result<bool> {
    let workspace_path = storage.get_workspace_path(workspace_id);
    let relative_path = path.strip_prefix('/').unwrap_or(path);
    // Note: workspace_path already includes /latest, don't add it again
    let file_path = workspace_path.join(relative_path);

    tracing::debug!(
        workspace_id = %workspace_id,
        path = %path,
        file_path = %file_path.display(),
        "Checking if file exists on disk"
    );

    let exists = tokio::fs::metadata(&file_path).await.is_ok();

    tracing::debug!(
        workspace_id = %workspace_id,
        path = %path,
        exists = exists,
        "File existence check result"
    );

    Ok(exists)
}

/// Read file content from disk
///
/// # Arguments
/// * `storage` - The file storage service
/// * `workspace_id` - The workspace ID
/// * `path` - The file path (with or without leading slash)
///
/// # Returns
/// * `Ok((content, hash))` - The file content and SHA-256 hash
/// * `Err(_)` if there was an error reading the file
pub async fn read_file_from_disk(
    storage: &FileStorageService,
    workspace_id: Uuid,
    path: &str,
) -> Result<(String, String)> {
    let workspace_path = storage.get_workspace_path(workspace_id);
    let relative_path = path.strip_prefix('/').unwrap_or(path);
    // Note: workspace_path already includes /latest, don't add it again
    let file_path = workspace_path.join(relative_path);

    tracing::debug!(
        workspace_id = %workspace_id,
        path = %path,
        file_path = %file_path.display(),
        "Reading file from disk"
    );

    let content = tokio::fs::read_to_string(&file_path).await?;

    // Calculate SHA-256 hash
    let hash = Sha256::digest(&content);
    let hash_hex = hex::encode(hash);

    tracing::debug!(
        workspace_id = %workspace_id,
        path = %path,
        content_length = content.len(),
        hash = %hash_hex,
        "Successfully read file from disk"
    );

    Ok((content, hash_hex))
}

/// Get file metadata from disk
///
/// # Arguments
/// * `storage` - The file storage service
/// * `workspace_id` - The workspace ID
/// * `path` - The file path (with or without leading slash)
///
/// # Returns
/// * `Ok((size, modified))` - The file size and modification time
/// * `Err(_)` if there was an error getting metadata
pub async fn get_file_metadata_from_disk(
    storage: &FileStorageService,
    workspace_id: Uuid,
    path: &str,
) -> Result<(usize, chrono::DateTime<chrono::Utc>)> {
    let workspace_path = storage.get_workspace_path(workspace_id);
    let relative_path = path.strip_prefix('/').unwrap_or(path);
    // Note: workspace_path already includes /latest, don't add it again
    let file_path = workspace_path.join(relative_path);

    let metadata = tokio::fs::metadata(&file_path).await?;

    let size = metadata.len() as usize;

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| {
                    chrono::DateTime::<chrono::Utc>::from_timestamp(
                        d.as_secs() as i64,
                        d.subsec_nanos(),
                    )
                })
        })
        .flatten()
        .unwrap_or_else(chrono::Utc::now);

    Ok((size, modified))
}

/// Extract filename from path
///
/// # Arguments
/// * `path` - The file path (with or without leading slash)
///
/// # Returns
/// The filename (last component of the path)
pub fn extract_filename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled")
        .to_string()
}

/// Extract parent ID from path
///
/// This function queries the database to find the parent folder ID.
/// If the path is at root (no parent), returns None.
///
/// # Arguments
/// * `conn` - Database connection
/// * `workspace_id` - The workspace ID
/// * `path` - The file path
///
/// # Returns
/// * `Ok(Some(parent_id))` - The parent folder ID
/// * `Ok(None)` - If the file is at root
/// * `Err(_)` if there was an error or parent not found
pub async fn extract_parent_id(
    conn: &mut DbConn,
    workspace_id: Uuid,
    path: &str,
) -> Result<Option<Uuid>> {
    // Extract parent path from file path
    // "/src/file.rs" -> "/src"
    // "/file.rs" -> "/"
    let parent_path = if let Some(idx) = path.rsplit_once('/') {
        if idx.0.is_empty() {
            None // Root
        } else {
            Some(idx.0.to_string())
        }
    } else {
        None // Root
    };

    // If root, return None
    let parent_path_str = match parent_path {
        None => return Ok(None),
        Some(p) => p,
    };

    // Query database for parent folder
    let parent_file = crate::queries::files::get_file_by_path(
        conn,
        workspace_id,
        &parent_path_str,
    )
    .await?;

    match parent_file {
        Some(file) => Ok(Some(file.id)),
        None => Err(crate::error::Error::NotFound(format!(
            "Parent folder not found: {}",
            parent_path_str
        ))),
    }
}

/// Create database entry from disk file
///
/// This function reads a file from disk and creates a corresponding database entry.
/// Useful for auto-importing files that were created externally.
///
/// # Arguments
/// * `conn` - Database connection
/// * `storage` - The file storage service
/// * `workspace_id` - The workspace ID
/// * `path` - The file path (with or without leading slash)
/// * `user_id` - The user ID who will be the author
///
/// # Returns
/// * `Ok(file)` - The created database file entry
/// * `Err(_)` if there was an error creating the entry
pub async fn import_file_to_database(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    path: &str,
    user_id: Uuid,
) -> Result<crate::models::files::File> {
    // Read file from disk
    let (content, _hash) = read_file_from_disk(storage, workspace_id, path).await?;

    // Determine file type (simple heuristic: check if it's a directory)
    let workspace_path = storage.get_workspace_path(workspace_id);
    let relative_path = path.strip_prefix('/').unwrap_or(path);
    let file_path = workspace_path.join("latest").join(relative_path);

    let file_type = if file_path.is_dir() {
        FileType::Folder
    } else {
        FileType::Document
    };

    // Extract filename and parent ID
    let name = extract_filename(path);
    let parent_id = extract_parent_id(conn, workspace_id, path).await?;

    // Create database entry
    let file_with_content = crate::services::files::create_file_with_content(
        conn,
        storage,
        CreateFileRequest {
            workspace_id,
            parent_id,
            author_id: user_id,
            name: name.clone(),
            slug: Some(name),
            path: Some(path.to_string()),
            is_virtual: Some(false),
            is_remote: Some(false),
            permission: None,
            file_type,
            content: serde_json::json!(content),
            app_data: None,
        },
    )
    .await?;

    Ok(file_with_content.file)
}

/// Delete file from disk only (not from database)
///
/// # Arguments
/// * `storage` - The file storage service
/// * `workspace_id` - The workspace ID
/// * `path` - The file path (with or without leading slash)
///
/// # Returns
/// * `Ok(())` if the file was deleted
/// * `Err(_)` if there was an error deleting the file
pub async fn delete_file_from_disk(
    storage: &FileStorageService,
    workspace_id: Uuid,
    path: &str,
) -> Result<()> {
    let workspace_path = storage.get_workspace_path(workspace_id);
    let relative_path = path.strip_prefix('/').unwrap_or(path);
    // Note: workspace_path already includes /latest, don't add it again
    let file_path = workspace_path.join(relative_path);

    tokio::fs::remove_file(&file_path).await?;
    Ok(())
}
