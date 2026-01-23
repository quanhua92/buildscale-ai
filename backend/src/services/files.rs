use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::{
        files::{File, FileStatus, FileType, NewFile, NewFileVersion},
        requests::{
            CreateFileRequest, CreateVersionRequest, FileNetworkSummary,
            FileWithContent, SearchResult, SemanticSearchHttp, UpdateFileHttp,
        },
    },
    queries::files,
    validation::validate_file_slug,
    config::AiConfig,
};
use pgvector::Vector;
use sha2::{Digest, Sha256};
use sqlx::Acquire;
use uuid::Uuid;

/// Hashes JSON content using SHA-256 for content-addressing
pub fn hash_content(content: &serde_json::Value) -> Result<String> {
    let content_str = serde_jcs::to_string(content)
        .map_err(|e| Error::Internal(format!("Failed to serialize to canonical JSON: {}", e)))?;
    let mut hasher = Sha256::new();
    hasher.update(content_str.as_bytes());
    let result = hasher.finalize();
    Ok(hex::encode(result))
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

    // Trim trailing separator
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
/// Returns the id of the last folder in the path.
pub async fn ensure_path_exists(
    conn: &mut DbConn,
    workspace_id: Uuid,
    path: &str,
    author_id: Uuid,
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

        // Check if folder exists at this path
        if let Some(file) = files::get_file_by_path(conn, workspace_id, &current_path_prefix).await? {
             if !matches!(file.file_type, FileType::Folder) {
                 return Err(Error::Conflict(format!("Path collision: '{}' is not a folder", current_path_prefix)));
             }
             current_parent_id = Some(file.id);
        } else {
            // Create folder
             let new_folder = NewFile {
                workspace_id,
                parent_id: current_parent_id,
                author_id,
                file_type: FileType::Folder,
                status: FileStatus::Ready,
                name: segment.to_string(),
                slug: slug.clone(),
                path: current_path_prefix.clone(),
            };
            let folder = files::create_file_identity(conn, new_folder).await?;
            current_parent_id = Some(folder.id);
        }
    }

    Ok(current_parent_id)
}

/// Creates a new file with its initial content version in a single transaction
pub async fn create_file_with_content(
    conn: &mut DbConn,
    request: CreateFileRequest,
) -> Result<FileWithContent> {
    // 1. Start transaction
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // 2. Resolve parent_id, slug, and path
    let (parent_id, name, slug, path) = if let Some(req_path) = request.path {
        // Path-based creation
        let req_path = req_path.trim().trim_matches('/');
        let (dir, filename) = match req_path.rsplit_once('/') {
            Some((d, f)) => (d, f),
            None => ("", req_path),
        };

        let parent_id = ensure_path_exists(&mut tx, request.workspace_id, dir, request.author_id).await?;
        let slug = slugify(filename);
        // Use provided name if valid, otherwise filename
        let name = if !request.name.trim().is_empty() { request.name } else { filename.to_string() };
        
        // Re-calculate path to be sure (canonical)
        let parent_path = if let Some(pid) = parent_id {
            let p_file = files::get_file_by_id(&mut tx, pid).await?;
            Some(p_file.path)
        } else {
            None
        };
        let final_path = calculate_path(parent_path.as_deref(), &slug);
        
        (parent_id, name, slug, final_path)
    } else {
        // Classic ID-based creation
        let name = request.name.trim().to_string();
        if name.is_empty() {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "name".to_string(),
                message: "File name cannot be empty".to_string(),
            }));
        }
        
        let slug = match request.slug {
            Some(s) => {
                let s = s.trim().to_lowercase();
                validate_file_slug(&s)?;
                s
            }
            None => {
                let s = slugify(&name);
                if s.is_empty() {
                    return Err(Error::Validation(crate::error::ValidationErrors::Single {
                        field: "name".to_string(),
                        message: "File name must contain alphanumeric characters to generate a valid URL slug".to_string(),
                    }));
                }
                s
            }
        };

        let parent_path = if let Some(pid) = request.parent_id {
             let p_file = files::get_file_by_id(&mut tx, pid).await?;
             Some(p_file.path)
        } else {
            None
        };
        let final_path = calculate_path(parent_path.as_deref(), &slug);

        (request.parent_id, name, slug, final_path)
    };

    // 3. Collision Check
    if files::get_file_by_path(&mut tx, request.workspace_id, &path).await?.is_some() {
        return Err(Error::Conflict(format!(
            "A file with path '{}' already exists",
            path
        )));
    }

    // 4. Create file identity record
    let new_file = NewFile {
        workspace_id: request.workspace_id,
        parent_id,
        author_id: request.author_id,
        file_type: request.file_type,
        status: FileStatus::Ready, // Set to Ready since we are providing content immediately
        name,
        slug,
        path,
    };
    let mut file = files::create_file_identity(&mut tx, new_file).await?;

    // 5. Calculate content hash
    let hash = hash_content(&request.content)?;

    // 6. Create first version record
    let new_version = NewFileVersion {
        file_id: file.id,
        workspace_id: file.workspace_id,
        branch: "main".to_string(),
        content_raw: request.content,
        app_data: request.app_data.unwrap_or(serde_json::json!({})),
        hash,
        author_id: Some(request.author_id),
    };
    let latest_version = files::create_version(&mut tx, new_version).await?;

    // 7. Update file with latest version ID cache
    files::update_latest_version_id(&mut tx, file.id, latest_version.id).await?;
    file.latest_version_id = Some(latest_version.id);

    // 8. Commit transaction
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
    let hash = hash_content(&request.content)?;

    // 1. Check if the latest version already has this hash (deduplication)
    let latest = files::get_latest_version_optional(conn, file_id).await?;
    if let Some(v) = latest.filter(|v| v.hash == hash) {
        return Ok(v);
    }

    // 2. Get file to obtain workspace_id
    let file = files::get_file_by_id(conn, file_id).await?;

    // 3. Start transaction
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // 4. Insert new version
    let new_version = NewFileVersion {
        file_id,
        workspace_id: file.workspace_id,
        branch: request.branch.unwrap_or_else(|| "main".to_string()),
        content_raw: request.content,
        app_data: request.app_data.unwrap_or(serde_json::json!({})),
        hash,
        author_id: request.author_id,
    };

    let version = files::create_version(&mut tx, new_version).await?;

    // 5. Update cache
    files::update_latest_version_id(&mut tx, file_id, version.id).await?;

    // 6. Commit
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(version)
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
    // Handle tri-state parent_id
    let target_parent_id = match request.parent_id {
        Some(new_parent) => new_parent, // Some(Some(uuid)) or Some(None) for root
        None => current_file.parent_id, // Field omitted
    };
    
    let target_name = request.name.as_deref().unwrap_or(&current_file.name).trim().to_string();
    if target_name.is_empty() {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "name".to_string(),
            message: "File name cannot be empty".to_string(),
        }));
    }

    let target_slug = if let Some(s) = request.slug {
        let s = s.trim().to_lowercase();
        validate_file_slug(&s)?;
        s
    } else if request.name.is_some() {
        // Name changed, update slug
        slugify(&target_name)
    } else {
        // Nothing changed regarding name/slug
        current_file.slug.clone()
    };

    // 3. Start transaction for complex check and update
    let mut tx = conn.begin().await.map_err(|e| {
        Error::Internal(format!("Failed to begin transaction: {}", e))
    })?;

    // 4. Calculate new path
    let parent_path = if let Some(pid) = target_parent_id {
         let p_file = files::get_file_by_id(&mut tx, pid).await?;
         Some(p_file.path)
    } else {
        None
    };
    let target_path = calculate_path(parent_path.as_deref(), &target_slug);

    // 5. Check if anything actually changed
    if target_parent_id == current_file.parent_id 
        && target_name == current_file.name 
        && target_slug == current_file.slug 
        && target_path == current_file.path
    {
        return Ok(current_file);
    }

    // 6. Cycle Detection (if moving)
    // Optimized: Check if new path starts with old path
    // e.g. Moving /A to /A/B -> New path /A/B starts with /A -> Error
    if current_file.file_type == FileType::Folder && target_path.starts_with(&format!("{}/", current_file.path)) {
         return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "parent_id".to_string(),
            message: "Cannot move a folder into one of its own subfolders".to_string(),
        }));
    }

    // 7. Collision Check
    // We exclude the current file from collision check (in case of rename to same name/path? filtered in step 5)
    if target_path != current_file.path {
        if files::get_file_by_path(&mut tx, current_file.workspace_id, &target_path).await?.is_some() {
            return Err(Error::Conflict(format!(
                "A file with path '{}' already exists",
                target_path
            )));
        }
    }

    // 8. Update metadata
    let updated_file = files::update_file_metadata(
        &mut tx, 
        file_id, 
        target_parent_id, 
        &target_name, 
        &target_slug,
        &target_path
    ).await?;

    // 9. If folder, update descendants (REBASE)
    if current_file.file_type == FileType::Folder && current_file.path != target_path {
        files::update_descendant_paths(
            &mut tx, 
            current_file.workspace_id, 
            &current_file.path, 
            &target_path
        ).await?;
    }

    // 10. Commit
    tx.commit().await.map_err(|e| {
        Error::Internal(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(updated_file)
}

/// Soft deletes a file with a check for empty folders
pub async fn soft_delete_file(conn: &mut DbConn, file_id: Uuid) -> Result<()> {
    let file = files::get_file_by_id(conn, file_id).await?;

    // If it's a folder, ensure it's empty
    if matches!(file.file_type, FileType::Folder) && files::has_active_children(conn, file_id).await? {
        return Err(Error::Conflict(
            "Cannot delete folder because it is not empty. Please delete or move sub-items first.".to_string(),
        ));
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

    let file = files::get_file_by_id(conn, file_id).await?;
    files::add_tag(conn, file_id, file.workspace_id, &tag).await
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

    files::add_link(conn, source_id, target_id, source.workspace_id).await
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

// ============================================================================
// AI & SEMANTIC SERVICES
// ============================================================================

/// Orchestrates the AI ingestion pipeline for a file.
pub async fn process_file_for_ai(
    conn: &mut DbConn,
    file_id: Uuid,
    ai_config: &AiConfig,
) -> Result<()> {
    // 1. Get file and its latest version
    let file = files::get_file_by_id(conn, file_id).await?;
    let latest_version = files::get_latest_version(conn, file_id).await?;

    // 2. Set status to Processing
    files::update_file_status(conn, file_id, FileStatus::Processing).await?;

    // 3. Process with error handling to avoid stuck status
    let process_result: Result<()> = async {
        // Extract text content (from JSONB)
        let text = extract_text_recursively(&latest_version.content_raw);

        if text.trim().is_empty() {
            return Ok(());
        }

        // 4. Chunk text
        let chunks = chunk_text(&text, ai_config.chunk_window_size, ai_config.chunk_overlap);

        // 5. Upsert chunks and link them
        for (i, chunk_text) in chunks.into_iter().enumerate() {
            // Hashing for semantic deduplication
            let mut hasher = Sha256::new();
            hasher.update(chunk_text.as_bytes());
            let chunk_hash = hex::encode(hasher.finalize());

            // Placeholder embedding: until OpenAI is integrated, we store a dummy vector
            // Dimension is configured in ai_config
            // Using a non-zero vector to avoid NaN similarity results
            let dummy_vector = Vector::from(vec![0.1; ai_config.embedding_dimension]);

            let chunk = files::upsert_chunk(
                conn,
                file.workspace_id,
                &chunk_hash,
                &chunk_text,
                dummy_vector,
            )
            .await?;

            files::link_version_to_chunk(
                conn,
                latest_version.id,
                chunk.id,
                file.workspace_id,
                i as i32,
            )
            .await?;
        }
        Ok(())
    }
    .await;

    // 6. Update final status
    match process_result {
        Ok(_) => {
            files::update_file_status(conn, file_id, FileStatus::Ready).await?;
            Ok(())
        }
        Err(e) => {
            // Log error or at least mark as failed
            files::update_file_status(conn, file_id, FileStatus::Failed).await?;
            Err(e)
        }
    }
}

/// Splits text into overlapping semantic windows.
pub fn chunk_text(text: &str, window_size: usize, overlap: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    if window_size == 0 {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();

    let mut start = 0;
    while start < n {
        let end = (start + window_size).min(n);
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);

        if end == n {
            break;
        }

        // Advance by window_size minus overlap
        let advance = window_size.saturating_sub(overlap).max(1);
        start += advance;
    }

    chunks
}

/// Recursively extracts all string values from a JSON structure.
/// This preserves some order by traversing arrays and objects sequentially.
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

/// Performs semantic search across the workspace.
pub async fn semantic_search(
    conn: &mut DbConn,
    workspace_id: Uuid,
    request: SemanticSearchHttp,
) -> Result<Vec<SearchResult>> {
    let limit = request.limit.unwrap_or(5).min(50);
    let query_vector = Vector::from(request.query_vector);

    let raw_results = files::semantic_search(conn, workspace_id, query_vector, limit).await?;

    let results = raw_results
        .into_iter()
        .map(|r| SearchResult {
            file: File {
                id: r.id,
                workspace_id: r.workspace_id,
                parent_id: r.parent_id,
                author_id: r.author_id,
                file_type: r.file_type,
                status: r.status,
                name: r.name,
                slug: r.slug,
                path: r.path,
                latest_version_id: r.latest_version_id,
                deleted_at: r.deleted_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
            chunk_content: r.chunk_content,
            similarity: r.similarity.unwrap_or(0.0) as f32,
        })
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_basic() {
        let text = "abcdefghij"; // 10 chars
        let chunks = chunk_text(text, 4, 2);
        // "abcd"
        //   "cdef"
        //     "efgh"
        //       "ghij"
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0], "abcd");
        assert_eq!(chunks[1], "cdef");
        assert_eq!(chunks[2], "efgh");
        assert_eq!(chunks[3], "ghij");
    }

    #[test]
    fn test_chunk_text_overlap_greater_than_window() {
        let text = "abc";
        let chunks = chunk_text(text, 2, 5); // overlap 5 > window 2
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], "ab");
        assert_eq!(chunks[1], "bc");
    }

    #[test]
    fn test_chunk_text_empty() {
        let chunks = chunk_text("", 10, 2);
        assert!(chunks.is_empty());
    }
}
