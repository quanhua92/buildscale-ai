use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::requests::{CreateFileRequest, FileWithContent},
};
use super::models::{File, FileType, NewFile};
use super::queries as file_queries;
use super::storage::FileStorageService;
use sha2::{Digest, Sha256};
use sqlx::Acquire;
use uuid::Uuid;

/// Hashes content using SHA-256 for content-addressing.
pub fn hash_content(content: &serde_json::Value) -> String {
    let bytes = match content {
        serde_json::Value::String(s) => s.as_bytes().to_vec(),
        _ => serde_json::to_string(content)
            .unwrap_or_default()
            .into_bytes(),
    };

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hex::encode(hasher.finalize())
}

/// Converts a display name into a URL-safe slug.
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_was_separator = true;

    for c in name.chars() {
        if c.is_alphanumeric() || c == '.' || c == '_' {
            slug.push(c.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            slug.push('-');
            last_was_separator = true;
        }
    }

    if slug.ends_with('-') || slug.ends_with('.') || slug.ends_with('_') {
        slug.pop();
    }

    slug
}

/// Helper to construct full path
pub fn calculate_path(parent_path: Option<&str>, slug: &str) -> String {
    match parent_path {
        Some(p) => format!("{}/{}", p.trim_end_matches('/'), slug),
        None => format!("/{}", slug),
    }
}

/// Recursively creates folders to ensure a path exists.
pub async fn ensure_path_exists(
    conn: &mut DbConn,
    workspace_id: Uuid,
    path: &str,
) -> Result<Option<Uuid>> {
    let path = path.trim().trim_matches('/');
    if path.is_empty() {
        return Ok(None);
    }

    let segments: Vec<&str> = path.split('/').collect();
    let mut current_parent_id: Option<Uuid> = None;
    let mut current_path_prefix = String::new();

    for segment in segments {
        let slug = slugify(segment);
        current_path_prefix.push('/');
        current_path_prefix.push_str(&slug);

        if let Some(file) = file_queries::get_file_by_path(conn, workspace_id, &current_path_prefix).await? {
            if !matches!(file.file_type, FileType::Folder) {
                return Err(Error::Conflict(format!("Path collision: '{}' is not a folder", current_path_prefix)));
            }
            current_parent_id = Some(file.id);
        } else {
            let new_folder = NewFile {
                workspace_id,
                parent_id: current_parent_id,
                file_type: FileType::Folder,
                name: segment.to_string(),
                path: current_path_prefix.clone(),
                hash: None,
            };
            let folder = file_queries::create_file(conn, new_folder).await?;
            current_parent_id = Some(folder.id);
        }
    }

    Ok(current_parent_id)
}

/// Creates a new file with content in a single transaction
pub async fn create_file_with_content(
    conn: &mut DbConn,
    storage: &FileStorageService,
    request: CreateFileRequest,
) -> Result<FileWithContent> {
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // Resolve parent_id, name, and path
    let (parent_id, name, path) = if let Some(req_path) = request.path {
        let req_path = req_path.trim().trim_matches('/');
        let (dir, filename) = match req_path.rsplit_once('/') {
            Some((d, f)) => (d, f),
            None => ("", req_path),
        };

        let parent_id = ensure_path_exists(&mut tx, request.workspace_id, dir).await?;
        let slug = slugify(filename);
        let name = if !request.name.trim().is_empty() { request.name } else { filename.to_string() };

        let parent_path = if let Some(pid) = parent_id {
            let p_file = file_queries::get_file_by_id(&mut tx, pid).await?;
            Some(p_file.path)
        } else {
            None
        };
        let final_path = calculate_path(parent_path.as_deref(), &slug);

        (parent_id, name, final_path)
    } else {
        let name = request.name.trim().to_string();
        if name.is_empty() {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "name".to_string(),
                message: "File name cannot be empty".to_string(),
            }));
        }

        let slug = slugify(&name);
        if slug.is_empty() {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "name".to_string(),
                message: "File name must contain alphanumeric characters to generate a valid URL slug".to_string(),
            }));
        }

        let parent_path = if let Some(pid) = request.parent_id {
            let p_file = file_queries::get_file_by_id(&mut tx, pid).await?;
            Some(p_file.path)
        } else {
            None
        };
        let final_path = calculate_path(parent_path.as_deref(), &slug);

        (request.parent_id, name, final_path)
    };

    // Collision Check
    if file_queries::get_file_by_path(&mut tx, request.workspace_id, &path).await?.is_some() {
        return Err(Error::Conflict(format!(
            "A file with path '{}' already exists",
            path
        )));
    }

    // Prepare content
    let content = request.content;
    let content_bytes = match &content {
        serde_json::Value::String(s) => s.as_bytes().to_vec(),
        _ => serde_json::to_vec(&content)
            .map_err(|e| Error::Internal(format!("Failed to serialize content: {}", e)))?,
    };

    let hash = hash_content(&content);

    // Create file identity
    let new_file = NewFile {
        workspace_id: request.workspace_id,
        parent_id,
        file_type: request.file_type,
        name,
        path: path.clone(),
        hash: Some(hash.clone()),
    };
    let file = file_queries::create_file(&mut tx, new_file).await?;

    // Write to storage
    if matches!(file.file_type, FileType::Folder) {
        storage.create_folder(file.workspace_id, &file.path).await?;
    } else {
        storage.write_file_with_hash(file.workspace_id, &file.path, &content_bytes, &hash).await?;
    }

    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(FileWithContent {
        file,
        hash,
        content,
    })
}

/// Updates file content (creates new version)
pub async fn update_file_content(
    conn: &mut DbConn,
    storage: &FileStorageService,
    file_id: Uuid,
    content: serde_json::Value,
) -> Result<File> {
    let file = file_queries::get_file_by_id(conn, file_id).await?;

    let content_bytes = match &content {
        serde_json::Value::String(s) => s.as_bytes().to_vec(),
        _ => serde_json::to_vec(&content)
            .map_err(|e| Error::Internal(format!("Failed to serialize content: {}", e)))?,
    };

    let new_hash = hash_content(&content);
    let old_hash = file.hash.clone();

    // Write to storage (archive old, write new)
    storage.write_file_with_hash(file.workspace_id, &file.path, &content_bytes, &new_hash).await?;

    // Update hash in database (appends old hash to versions array)
    let updated_file = file_queries::update_file_hash(conn, file_id, &new_hash, old_hash.as_deref()).await?;

    Ok(updated_file)
}

/// Gets a file and its content
pub async fn get_file_with_content(
    conn: &mut DbConn,
    storage: &FileStorageService,
    file_id: Uuid
) -> Result<FileWithContent> {
    let file = file_queries::get_file_by_id(conn, file_id).await?;

    let content = match &file.hash {
        Some(hash) => {
            match storage.read_file(file.workspace_id, &file.path).await {
                Ok(bytes) => {
                    serde_json::from_slice(&bytes)
                        .unwrap_or_else(|_| {
                            serde_json::Value::String(String::from_utf8_lossy(&bytes).to_string())
                        })
                },
                Err(Error::NotFound(_)) => {
                    // Try to restore from archive
                    match storage.read_version(file.workspace_id, hash).await {
                        Ok(bytes) => {
                            // Heal: Write back to working tree
                            let _ = storage.write_latest_file(file.workspace_id, &file.path, &bytes).await;
                            serde_json::from_slice(&bytes)
                                .unwrap_or_else(|_| {
                                    serde_json::Value::String(String::from_utf8_lossy(&bytes).to_string())
                                })
                        },
                        Err(_) => {
                            tracing::error!("File content missing on disk and archive for file {}", file.path);
                            serde_json::json!({"error": "Content missing"})
                        }
                    }
                },
                Err(e) => return Err(e),
            }
        },
        None => serde_json::json!(null),
    };

    Ok(FileWithContent {
        hash: file.hash.clone().unwrap_or_default(),
        file,
        content,
    })
}

/// Restores a specific version
pub async fn restore_version(
    conn: &mut DbConn,
    storage: &FileStorageService,
    file_id: Uuid,
    version_hash: &str,
) -> Result<File> {
    let file = file_queries::get_file_by_id(conn, file_id).await?;

    // Read content from archive
    let bytes = storage.read_version(file.workspace_id, version_hash).await?;

    // Write to latest
    storage.write_latest_file(file.workspace_id, &file.path, &bytes).await?;

    // Update database (old hash goes to versions)
    let updated_file = file_queries::restore_version(conn, file_id, version_hash).await?;

    Ok(updated_file)
}

/// Updates a file's metadata (move, rename)
pub async fn update_file(
    conn: &mut DbConn,
    storage: &FileStorageService,
    file_id: Uuid,
    name: Option<String>,
    parent_id: Option<Option<Uuid>>,
) -> Result<File> {
    let current_file = file_queries::get_file_by_id(conn, file_id).await?;

    let target_parent_id = match parent_id {
        Some(new_parent) => new_parent,
        None => current_file.parent_id,
    };

    let target_name = name.as_deref().unwrap_or(&current_file.name).trim().to_string();
    if target_name.is_empty() {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "name".to_string(),
            message: "File name cannot be empty".to_string(),
        }));
    }

    let target_slug = slugify(&target_name);

    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    let parent_path = if let Some(pid) = target_parent_id {
        let p_file = file_queries::get_file_by_id(&mut tx, pid).await?;
        Some(p_file.path)
    } else {
        None
    };
    let target_path = calculate_path(parent_path.as_deref(), &target_slug);

    // Check if anything changed
    if target_parent_id == current_file.parent_id
        && target_name == current_file.name
        && target_path == current_file.path
    {
        return Ok(current_file);
    }

    // Cycle Detection
    if current_file.file_type == FileType::Folder && target_path.starts_with(&format!("{}/", current_file.path)) {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "parent_id".to_string(),
            message: "Cannot move a folder into one of its own subfolders".to_string(),
        }));
    }

    // Collision Check
    if target_path != current_file.path
        && file_queries::get_file_by_path(&mut tx, current_file.workspace_id, &target_path).await?.is_some()
    {
        return Err(Error::Conflict(format!(
            "A file with path '{}' already exists",
            target_path
        )));
    }

    // Update metadata
    let updated_file = file_queries::update_file_metadata(
        &mut tx,
        file_id,
        target_parent_id,
        &target_name,
        &target_path,
    ).await?;

    // Update descendants if folder
    if current_file.file_type == FileType::Folder && current_file.path != target_path {
        file_queries::update_descendant_paths(
            &mut tx,
            current_file.workspace_id,
            &current_file.path,
            &target_path
        ).await?;
    }

    // Move on disk
    if current_file.path != target_path {
        storage.move_file(current_file.workspace_id, &current_file.path, &target_path).await?;
    }

    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(updated_file)
}

/// Soft deletes a file
pub async fn soft_delete_file(
    conn: &mut DbConn,
    storage: &FileStorageService,
    file_id: Uuid
) -> Result<()> {
    let file = file_queries::get_file_by_id(conn, file_id).await?;

    if file.deleted_at.is_some() {
        return Err(Error::NotFound(format!("File already deleted: {}", file.path)));
    }

    if matches!(file.file_type, FileType::Folder) {
        let has_children = file_queries::has_active_children(conn, file_id).await?;
        let has_descendants = file_queries::has_active_descendants(conn, file.workspace_id, &file.path).await?;

        if has_children || has_descendants {
            return Err(Error::Conflict(
                "Cannot delete folder because it is not empty. Please delete or move sub-items first.".to_string(),
            ));
        }
    }

    if !matches!(file.file_type, FileType::Folder) {
        storage.move_to_trash(file.workspace_id, &file.path).await?;
    }

    let rows_affected = file_queries::soft_delete_file(conn, file_id).await?;

    if rows_affected != 1 {
        return Err(Error::Internal(format!(
            "Critical safety error: Expected to delete exactly 1 file, but affected {} rows.",
            rows_affected
        )));
    }

    Ok(())
}

/// Restores a soft-deleted file
pub async fn restore_file(
    conn: &mut DbConn,
    storage: &FileStorageService,
    file_id: Uuid
) -> Result<File> {
    let file = file_queries::get_file_by_id(conn, file_id).await?;

    if file.deleted_at.is_none() {
        return Ok(file);
    }

    // Collision check
    if file_queries::check_path_collision(conn, file.workspace_id, &file.path).await? {
        return Err(Error::Conflict(format!(
            "Cannot restore '{}' because another file with the same path already exists.",
            file.path
        )));
    }

    // Restore from archive if needed
    if let Some(hash) = &file.hash {
        storage.ensure_file_restored(file.workspace_id, &file.path, hash).await?;
    }

    file_queries::restore_file(conn, file_id).await
}

/// Hard deletes a file (Purge)
pub async fn purge_file(conn: &mut DbConn, workspace_id: Uuid, file_id: Uuid) -> Result<Vec<String>> {
    let file = file_queries::get_file_by_id(conn, file_id).await?;

    // Collect all hashes for cleanup
    let mut hashes = Vec::new();
    if let Some(h) = file.hash {
        hashes.push(h);
    }
    if let Some(v) = file.versions {
        hashes.extend(v);
    }

    file_queries::hard_delete_file(conn, workspace_id, file_id).await?;

    Ok(hashes)
}

/// Lists all items in the trash
pub async fn list_trash(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<File>> {
    file_queries::list_trash(conn, workspace_id).await
}

/// Gets all active (non-deleted) files in a workspace.
pub async fn list_all_active_files(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<File>> {
    file_queries::list_all_active_files(conn, workspace_id).await
}

/// Recursively extracts all string values from a JSON structure.
pub fn extract_text_recursively(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(extract_text_recursively)
            .collect::<Vec<_>>()
            .join("\n"),
        serde_json::Value::Object(obj) => obj
            .values()
            .map(extract_text_recursively)
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Gets files by a list of IDs.
pub async fn get_files_by_ids(conn: &mut DbConn, file_ids: &[Uuid]) -> Result<Vec<File>> {
    if file_ids.is_empty() {
        return Ok(vec![]);
    }

    file_queries::get_files_by_ids(conn, file_ids).await
}
