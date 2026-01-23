use crate::{
    error::{Error, Result},
    models::files::{File, FileChunk, FileStatus, FileType, FileVersion, NewFile, NewFileVersion},
    DbConn,
};
use pgvector::Vector;
use uuid::Uuid;

/// Creates a new file identity in the database.
pub async fn create_file_identity(conn: &mut DbConn, new_file: NewFile) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        INSERT INTO files (workspace_id, parent_id, author_id, file_type, status, slug)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING 
            id, 
            workspace_id, 
            parent_id, 
            author_id, 
            file_type as "file_type: FileType", 
            status as "status: FileStatus", 
            slug, 
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        "#,
        new_file.workspace_id,
        new_file.parent_id,
        new_file.author_id,
        new_file.file_type as FileType,
        new_file.status as FileStatus,
        new_file.slug
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(file)
}

/// Creates a new file version in the database.
pub async fn create_version(conn: &mut DbConn, new_version: NewFileVersion) -> Result<FileVersion> {
    let version = sqlx::query_as!(
        FileVersion,
        r#"
        INSERT INTO file_versions (file_id, workspace_id, branch, content_raw, app_data, hash, author_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING 
            id, 
            file_id, 
            workspace_id,
            branch as "branch!", 
            content_raw, 
            app_data, 
            hash, 
            author_id as "author_id?", 
            created_at, 
            updated_at
        "#,
        new_version.file_id,
        new_version.workspace_id,
        new_version.branch,
        new_version.content_raw,
        new_version.app_data,
        new_version.hash,
        new_version.author_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(version)
}

/// Updates the latest version ID for a file.
pub async fn update_latest_version_id(
    conn: &mut DbConn,
    file_id: Uuid,
    version_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE files SET latest_version_id = $2, updated_at = NOW() WHERE id = $1
        "#,
        file_id,
        version_id
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}

/// Gets a file by its ID.
pub async fn get_file_by_id(conn: &mut DbConn, id: Uuid) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        SELECT 
            id, 
            workspace_id, 
            parent_id, 
            author_id, 
            file_type as "file_type: FileType", 
            status as "status: FileStatus", 
            slug, 
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        FROM files
        WHERE id = $1
        "#,
        id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(file)
}

/// Gets the latest version of a file.
pub async fn get_latest_version(conn: &mut DbConn, file_id: Uuid) -> Result<FileVersion> {
    let version = sqlx::query_as!(
        FileVersion,
        r#"
        SELECT 
            fv.id, 
            fv.file_id, 
            fv.workspace_id,
            fv.branch as "branch!", 
            fv.content_raw, 
            fv.app_data, 
            fv.hash, 
            fv.author_id as "author_id?", 
            fv.created_at, 
            fv.updated_at
        FROM file_versions fv
        INNER JOIN files f ON fv.id = f.latest_version_id
        WHERE f.id = $1
        "#,
        file_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(version)
}

/// Gets the latest version of a file (optional).
pub async fn get_latest_version_optional(
    conn: &mut DbConn,
    file_id: Uuid,
) -> Result<Option<FileVersion>> {
    let version = sqlx::query_as!(
        FileVersion,
        r#"
        SELECT 
            fv.id, 
            fv.file_id, 
            fv.workspace_id,
            fv.branch as "branch!", 
            fv.content_raw, 
            fv.app_data, 
            fv.hash, 
            fv.author_id as "author_id?", 
            fv.created_at, 
            fv.updated_at
        FROM file_versions fv
        INNER JOIN files f ON fv.id = f.latest_version_id
        WHERE f.id = $1
        "#,
        file_id
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(version)
}

/// Resolves a file by its slug and parent_id.
pub async fn get_file_by_slug(
    conn: &mut DbConn,
    workspace_id: Uuid,
    parent_id: Option<Uuid>,
    slug: &str,
) -> Result<Option<File>> {
    let file = sqlx::query_as!(
        File,
        r#"
        SELECT 
            id, 
            workspace_id, 
            parent_id, 
            author_id, 
            file_type as "file_type: FileType", 
            status as "status: FileStatus", 
            slug, 
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        FROM files
        WHERE workspace_id = $1 
          AND (parent_id = $2 OR (parent_id IS NULL AND $2 IS NULL))
          AND slug = $3
          AND deleted_at IS NULL
        "#,
        workspace_id,
        parent_id,
        slug
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(file)
}

/// Lists all active files in a workspace/folder.
pub async fn list_files_in_folder(
    conn: &mut DbConn,
    workspace_id: Uuid,
    parent_id: Option<Uuid>,
) -> Result<Vec<File>> {
    let files = sqlx::query_as!(
        File,
        r#"
        SELECT 
            id, 
            workspace_id, 
            parent_id, 
            author_id, 
            file_type as "file_type: FileType", 
            status as "status: FileStatus", 
            slug, 
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        FROM files
        WHERE workspace_id = $1 
          AND (parent_id = $2 OR (parent_id IS NULL AND $2 IS NULL))
          AND deleted_at IS NULL
        ORDER BY (file_type = 'folder') DESC, slug ASC
        "#,
        workspace_id,
        parent_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(files)
}

// ============================================================================
// ORGANIZATION & HIERARCHY QUERIES
// ============================================================================

/// Checks if a file has any active (not deleted) children.
pub async fn has_active_children(conn: &mut DbConn, file_id: Uuid) -> Result<bool> {
    let result = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM files 
            WHERE parent_id = $1 AND deleted_at IS NULL
        ) as "exists!"
        "#,
        file_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.exists)
}

/// Checks if a slug collision exists in a target folder.
pub async fn check_slug_collision(
    conn: &mut DbConn,
    workspace_id: Uuid,
    parent_id: Option<Uuid>,
    slug: &str,
) -> Result<bool> {
    let result = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM files 
            WHERE workspace_id = $1 
              AND (parent_id = $2 OR (parent_id IS NULL AND $2 IS NULL))
              AND slug = $3
              AND deleted_at IS NULL
        ) as "exists!"
        "#,
        workspace_id,
        parent_id,
        slug
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.exists)
}

/// Checks if one file is a descendant of another using a recursive CTE.
pub async fn is_descendant_of(
    conn: &mut DbConn,
    potential_descendant_id: Uuid,
    potential_ancestor_id: Uuid,
) -> Result<bool> {
    let result = sqlx::query!(
        r#"
        WITH RECURSIVE file_ancestry AS (
            SELECT id, parent_id FROM files WHERE id = $1
            UNION ALL
            SELECT f.id, f.parent_id FROM files f
            INNER JOIN file_ancestry fa ON f.id = fa.parent_id
        )
        SELECT EXISTS(
            SELECT 1 FROM file_ancestry WHERE id = $2
        ) as "exists!"
        "#,
        potential_descendant_id,
        potential_ancestor_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.exists)
}

/// Updates file metadata (parent_id and/or slug).
pub async fn update_file_metadata(
    conn: &mut DbConn,
    file_id: Uuid,
    parent_id: Option<Uuid>,
    slug: &str,
) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        UPDATE files
        SET parent_id = $2, slug = $3, updated_at = NOW()
        WHERE id = $1
        RETURNING 
            id, 
            workspace_id, 
            parent_id, 
            author_id, 
            file_type as "file_type: FileType", 
            status as "status: FileStatus", 
            slug, 
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        "#,
        file_id,
        parent_id,
        slug
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(file)
}

/// Performs a soft delete on a file.
pub async fn soft_delete_file(conn: &mut DbConn, file_id: Uuid) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE files
        SET deleted_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
        file_id
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}

/// Restores a soft-deleted file.
pub async fn restore_file(conn: &mut DbConn, file_id: Uuid) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        UPDATE files
        SET deleted_at = NULL, updated_at = NOW()
        WHERE id = $1
        RETURNING 
            id, 
            workspace_id, 
            parent_id, 
            author_id, 
            file_type as "file_type: FileType", 
            status as "status: FileStatus", 
            slug, 
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        "#,
        file_id
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(file)
}

/// Lists all soft-deleted files in a workspace.
pub async fn list_trash(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<File>> {
    let files = sqlx::query_as!(
        File,
        r#"
        SELECT 
            id, 
            workspace_id, 
            parent_id, 
            author_id, 
            file_type as "file_type: FileType", 
            status as "status: FileStatus", 
            slug, 
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        FROM files
        WHERE workspace_id = $1 
          AND deleted_at IS NOT NULL
        ORDER BY deleted_at DESC
        "#,
        workspace_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(files)
}

// ============================================================================
// TAGGING QUERIES
// ============================================================================

/// Adds a tag to a file.
pub async fn add_tag(conn: &mut DbConn, file_id: Uuid, workspace_id: Uuid, tag: &str) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO file_tags (file_id, workspace_id, tag)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
        file_id,
        workspace_id,
        tag
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}

/// Removes a tag from a file.
pub async fn remove_tag(conn: &mut DbConn, file_id: Uuid, tag: &str) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM file_tags
        WHERE file_id = $1 AND tag = $2
        "#,
        file_id,
        tag
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}

/// Gets all tags for a file.
pub async fn get_tags_for_file(conn: &mut DbConn, file_id: Uuid) -> Result<Vec<String>> {
    let tags = sqlx::query!(
        r#"
        SELECT tag FROM file_tags
        WHERE file_id = $1
        ORDER BY tag ASC
        "#,
        file_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?
    .into_iter()
    .map(|r| r.tag)
    .collect();

    Ok(tags)
}

/// Lists files by tag in a workspace.
pub async fn list_files_by_tag(
    conn: &mut DbConn,
    workspace_id: Uuid,
    tag: &str,
) -> Result<Vec<File>> {
    let files = sqlx::query_as!(
        File,
        r#"
        SELECT 
            f.id, 
            f.workspace_id, 
            f.parent_id, 
            f.author_id, 
            f.file_type as "file_type: FileType", 
            f.status as "status: FileStatus", 
            f.slug, 
            f.latest_version_id,
            f.deleted_at, 
            f.created_at, 
            f.updated_at
        FROM files f
        INNER JOIN file_tags ft ON f.id = ft.file_id
        WHERE f.workspace_id = $1 
          AND ft.tag = $2
          AND f.deleted_at IS NULL
        ORDER BY f.updated_at DESC
        "#,
        workspace_id,
        tag
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(files)
}

// ============================================================================
// LINKING QUERIES
// ============================================================================

/// Adds a link between two files.
pub async fn add_link(
    conn: &mut DbConn,
    source_id: Uuid,
    target_id: Uuid,
    workspace_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO file_links (source_file_id, target_file_id, workspace_id)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
        source_id,
        target_id,
        workspace_id
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}

/// Removes a link between two files.
pub async fn remove_link(conn: &mut DbConn, source_id: Uuid, target_id: Uuid) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM file_links
        WHERE source_file_id = $1 AND target_file_id = $2
        "#,
        source_id,
        target_id
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}

/// Gets all files that a specific file links TO.
pub async fn get_outbound_links(conn: &mut DbConn, file_id: Uuid) -> Result<Vec<File>> {
    let files = sqlx::query_as!(
        File,
        r#"
        SELECT 
            f.id, 
            f.workspace_id, 
            f.parent_id, 
            f.author_id, 
            f.file_type as "file_type: FileType", 
            f.status as "status: FileStatus", 
            f.slug, 
            f.latest_version_id,
            f.deleted_at, 
            f.created_at, 
            f.updated_at
        FROM files f
        INNER JOIN file_links fl ON f.id = fl.target_file_id
        WHERE fl.source_file_id = $1
          AND f.deleted_at IS NULL
        ORDER BY f.slug ASC
        "#,
        file_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(files)
}

/// Gets all files that link TO a specific file (backlinks).
pub async fn get_backlinks(conn: &mut DbConn, file_id: Uuid) -> Result<Vec<File>> {
    let files = sqlx::query_as!(
        File,
        r#"
        SELECT 
            f.id, 
            f.workspace_id, 
            f.parent_id, 
            f.author_id, 
            f.file_type as "file_type: FileType", 
            f.status as "status: FileStatus", 
            f.slug, 
            f.latest_version_id,
            f.deleted_at, 
            f.created_at, 
            f.updated_at
        FROM files f
        INNER JOIN file_links fl ON f.id = fl.source_file_id
        WHERE fl.target_file_id = $1
          AND f.deleted_at IS NULL
        ORDER BY f.slug ASC
        "#,
        file_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(files)
}

// ============================================================================
// AI & SEMANTIC QUERIES
// ============================================================================

/// Creates a new semantic chunk or returns existing one if hash matches.
pub async fn upsert_chunk(
    conn: &mut DbConn,
    workspace_id: Uuid,
    chunk_hash: &str,
    content: &str,
    embedding: Vector,
) -> Result<FileChunk> {
    let chunk = sqlx::query_as!(
        FileChunk,
        r#"
        INSERT INTO file_chunks (workspace_id, chunk_hash, chunk_content, embedding)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (workspace_id, chunk_hash) DO UPDATE 
        SET chunk_content = EXCLUDED.chunk_content, embedding = EXCLUDED.embedding
        RETURNING id, workspace_id, chunk_hash, chunk_content, embedding as "embedding: Vector", created_at
        "#,
        workspace_id,
        chunk_hash,
        content,
        embedding as _
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(chunk)
}

/// Links a file version to a semantic chunk.
pub async fn link_version_to_chunk(
    conn: &mut DbConn,
    version_id: Uuid,
    chunk_id: Uuid,
    workspace_id: Uuid,
    index: i32,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO file_version_chunks (file_version_id, chunk_id, workspace_id, chunk_index)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (file_version_id, chunk_index) DO UPDATE 
        SET chunk_id = EXCLUDED.chunk_id
        "#,
        version_id,
        chunk_id,
        workspace_id,
        index
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}

/// Performs semantic search within a workspace.
pub async fn semantic_search(
    conn: &mut DbConn,
    workspace_id: Uuid,
    query_vector: Vector,
    limit: i32,
) -> Result<Vec<SearchResultRow>> {
    // Note: cosine similarity = 1 - cosine distance
    // pgvector <=> is cosine distance
    // Optimized: uses latest_version_id cache and workspace_id for O(1) tenant lookup
    let results = sqlx::query_as!(
        SearchResultRow,
        r#"
        SELECT 
            f.id, f.workspace_id, f.parent_id, f.author_id, 
            f.file_type as "file_type: FileType", 
            f.status as "status: FileStatus", 
            f.slug, f.latest_version_id, f.deleted_at, f.created_at, f.updated_at,
            fc.chunk_content,
            (1 - (fc.embedding <=> $2)) as "similarity: f64"
        FROM file_chunks fc
        INNER JOIN file_version_chunks fvc ON fc.id = fvc.chunk_id AND fc.workspace_id = fvc.workspace_id
        INNER JOIN files f ON fvc.file_version_id = f.latest_version_id AND fvc.workspace_id = f.workspace_id
        WHERE fc.workspace_id = $1
          AND f.deleted_at IS NULL
        ORDER BY fc.embedding <=> $2
        LIMIT $3
        "#,
        workspace_id,
        query_vector as Vector,
        limit as i64
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(results)
}

/// Row structure for semantic search results.
#[derive(Debug, Clone)]
pub struct SearchResultRow {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub author_id: Option<Uuid>,
    pub file_type: FileType,
    pub status: FileStatus,
    pub slug: String,
    pub latest_version_id: Option<Uuid>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub chunk_content: String,
    pub similarity: Option<f64>,
}

/// Updates file status.
pub async fn update_file_status(conn: &mut DbConn, file_id: Uuid, status: FileStatus) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE files SET status = $2, updated_at = NOW() WHERE id = $1
        "#,
        file_id,
        status as FileStatus
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
}
