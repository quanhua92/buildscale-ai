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
        INSERT INTO files (workspace_id, parent_id, author_id, file_type, status, name, slug, path, is_virtual, is_remote, permission)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING 
            id, 
            workspace_id, 
            parent_id, 
            author_id, 
            file_type as "file_type: FileType", 
            status as "status: FileStatus", 
            name,
            slug, 
            path,
            is_virtual,
            is_remote,
            permission,
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
        new_file.name,
        new_file.slug,
        new_file.path,
        new_file.is_virtual,
        new_file.is_remote,
        new_file.permission
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

/// Updates the `updated_at` timestamp for a file.
pub async fn touch_file(conn: &mut DbConn, file_id: Uuid) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE files SET updated_at = NOW() WHERE id = $1
        "#,
        file_id
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
            name,
            slug, 
            path,
            is_virtual,
            is_remote,
            permission,
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
            name,
            slug, 
            path,
            is_virtual,
            is_remote,
            permission,
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
            name,
            slug, 
            path,
            is_virtual,
            is_remote,
            permission,
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        FROM files
        WHERE workspace_id = $1 
          AND (parent_id = $2 OR (parent_id IS NULL AND $2 IS NULL))
          AND deleted_at IS NULL
        ORDER BY (file_type = 'folder') DESC, name ASC
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

/// Resolves a file by its materialized path.
pub async fn get_file_by_path(
    conn: &mut DbConn,
    workspace_id: Uuid,
    path: &str,
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
            name,
            slug, 
            path,
            is_virtual,
            is_remote,
            permission,
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        FROM files
        WHERE workspace_id = $1 
          AND path = $2
          AND deleted_at IS NULL
        "#,
        workspace_id,
        path
    )
    .fetch_optional(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(file)
}

/// Updates paths for all descendants of a folder.
/// This acts as a "rebase" operation: replacing the old prefix with the new prefix.
pub async fn update_descendant_paths(
    conn: &mut DbConn,
    workspace_id: Uuid,
    old_path_prefix: &str,
    new_path_prefix: &str,
) -> Result<()> {
    // We add a trailing slash to avoid partial matches (e.g. /foo matching /food)
    // But we need to be careful with string concatenation in SQL
    let old_prefix_slash = format!("{}/", old_path_prefix);

    sqlx::query!(
        r#"
        UPDATE files
        SET path = $2 || SUBSTRING(path FROM LENGTH($3) + 1), updated_at = NOW()
        WHERE workspace_id = $1
          AND path LIKE $4 || '%'
        "#,
        workspace_id,
        new_path_prefix,
        old_path_prefix, // Use original prefix for length calculation
        old_prefix_slash // Use prefix with slash for LIKE matching
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(())
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

/// Updates file metadata (parent_id, name, slug, path, virtual status, permissions).
#[allow(clippy::too_many_arguments)]
pub async fn update_file_metadata(
    conn: &mut DbConn,
    file_id: Uuid,
    parent_id: Option<Uuid>,
    name: &str,
    slug: &str,
    path: &str,
    is_virtual: bool,
    is_remote: bool,
    permission: i32,
) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        UPDATE files
        SET parent_id = $2, name = $3, slug = $4, path = $5, is_virtual = $6, is_remote = $7, permission = $8, updated_at = NOW()
        WHERE id = $1
        RETURNING 
            id, 
            workspace_id, 
            parent_id, 
            author_id, 
            file_type as "file_type: FileType", 
            status as "status: FileStatus", 
            name,
            slug, 
            path,
            is_virtual,
            is_remote,
            permission,
            latest_version_id,
            deleted_at, 
            created_at, 
            updated_at
        "#,
        file_id,
        parent_id,
        name,
        slug,
        path,
        is_virtual,
        is_remote,
        permission
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
            name,
            slug, 
            path,
            is_virtual,
            is_remote,
            permission,
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
            name,
            slug, 
            path,
            is_virtual,
            is_remote,
            permission,
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
            f.name,
            f.slug, 
            f.path,
            f.is_virtual,
            f.is_remote,
            f.permission,
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
            f.name,
            f.slug, 
            f.path,
            f.is_virtual,
            f.is_remote,
            f.permission,
            f.latest_version_id,
            f.deleted_at, 
            f.created_at, 
            f.updated_at
        FROM files f
        INNER JOIN file_links fl ON f.id = fl.target_file_id
        WHERE fl.source_file_id = $1
          AND f.deleted_at IS NULL
        ORDER BY f.name ASC
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
            f.name,
            f.slug, 
            f.path,
            f.is_virtual,
            f.is_remote,
            f.permission,
            f.latest_version_id,
            f.deleted_at, 
            f.created_at, 
            f.updated_at
        FROM files f
        INNER JOIN file_links fl ON f.id = fl.source_file_id
        WHERE fl.target_file_id = $1
          AND f.deleted_at IS NULL
        ORDER BY f.name ASC
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
            f.name, f.slug, f.path, 
            f.is_virtual, f.is_remote, f.permission,
            f.latest_version_id, f.deleted_at, f.created_at, f.updated_at,
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

/// Performs a regex search across all document files in a workspace.
pub async fn grep_files(
    conn: &mut DbConn,
    workspace_id: Uuid,
    pattern: &str,
    path_pattern: Option<&str>,
    case_sensitive: bool,
) -> Result<Vec<crate::models::requests::GrepMatch>> {
    let results = sqlx::query_as!(
        crate::models::requests::GrepMatch,
        r#"
        SELECT 
            f.path as "path!",
            t.line_number::int4 as "line_number!",
            t.line_text as "line_text!"
        FROM files f
        JOIN file_versions fv ON f.latest_version_id = fv.id
        CROSS JOIN LATERAL unnest(string_to_array(
            CASE 
                WHEN fv.content_raw ? 'text' THEN fv.content_raw->>'text'
                WHEN jsonb_typeof(fv.content_raw) = 'string' THEN fv.content_raw #>> '{}'
                ELSE fv.content_raw::text 
            END, 
            E'\n'
        )) 
            WITH ORDINALITY AS t(line_text, line_number)
        WHERE f.workspace_id = $1
          AND f.deleted_at IS NULL
          AND f.file_type = 'document'
          AND ($3::text IS NULL OR f.path ILIKE $3::text)
          AND (CASE WHEN $4::boolean THEN t.line_text ~ $2 ELSE t.line_text ~* $2 END)
        ORDER BY f.path, t.line_number
        LIMIT 1000
        "#,
        workspace_id,
        pattern,
        path_pattern,
        case_sensitive
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
    pub name: String,
    pub slug: String,
    pub path: String,
    pub is_virtual: bool,
    pub is_remote: bool,
    pub permission: i32,
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
