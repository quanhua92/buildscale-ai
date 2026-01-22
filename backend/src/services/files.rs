use crate::DbConn;
use crate::{
    error::{Error, Result},
    models::{
        files::{FileStatus, NewFile, NewFileVersion},
        requests::{CreateFileRequest, CreateVersionRequest, FileWithContent},
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
