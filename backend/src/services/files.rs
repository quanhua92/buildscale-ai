use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::{
        files::{File, FileStatus, FileType, NewFile, NewFileVersion},
        requests::{CreateFileRequest, CreateVersionRequest, FileNetworkSummary, FileWithContent, SearchResult, SemanticSearchHttp, UpdateFileHttp},
    },
    queries::files,
    validation::validate_file_slug,
};
use pgvector::Vector;
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

// ============================================================================
// AI & SEMANTIC SERVICES
// ============================================================================

/// Orchestrates the AI ingestion pipeline for a file.
pub async fn process_file_for_ai(conn: &mut DbConn, file_id: Uuid) -> Result<()> {
    // 1. Get file and its latest version
    let file = files::get_file_by_id(conn, file_id).await?;
    let latest_version = files::get_latest_version(conn, file_id).await?;

    // 2. Set status to Processing
    files::update_file_status(conn, file_id, FileStatus::Processing).await?;

    // 3. Extract text content (from JSONB)
    let text = if let Some(content) = latest_version.content_raw.as_str() {
        content.to_string()
    } else if let Some(obj) = latest_version.content_raw.as_object() {
        // Fallback for structured JSON: flatten all strings
        obj.values()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // If it's not text or an object, we can't chunk it yet
        files::update_file_status(conn, file_id, FileStatus::Ready).await?;
        return Ok(());
    };

    if text.trim().is_empty() {
        files::update_file_status(conn, file_id, FileStatus::Ready).await?;
        return Ok(());
    }

    // 4. Chunk text
    let chunks = chunk_text(&text, 1000, 200);

    // 5. Upsert chunks and link them
    for (i, chunk_text) in chunks.into_iter().enumerate() {
        // Hashing for semantic deduplication
        let mut hasher = Sha256::new();
        hasher.update(chunk_text.as_bytes());
        let chunk_hash = hex::encode(hasher.finalize());

        // Placeholder embedding: until OpenAI is integrated, we store a dummy vector
        // Dimension is 1536 for text-embedding-3-small
        let dummy_vector = Vector::from(vec![0.0; 1536]);

        let chunk = files::upsert_chunk(
            conn,
            file.workspace_id,
            &chunk_hash,
            &chunk_text,
            dummy_vector,
        )
        .await?;

        files::link_version_to_chunk(conn, latest_version.id, chunk.id, i as i32).await?;
    }

    // 6. Final status: Ready
    files::update_file_status(conn, file_id, FileStatus::Ready).await?;

    Ok(())
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
        .map(|(file, chunk_content, similarity)| SearchResult {
            file,
            chunk_content,
            similarity,
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
