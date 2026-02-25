use crate::{
    error::{Error, Result},
    models::files::{File, FileType, NewFile},
    DbConn,
};
use uuid::Uuid;

/// Creates a new file in the database.
pub async fn create_file(conn: &mut DbConn, new_file: NewFile) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        INSERT INTO files (workspace_id, parent_id, file_type, name, path, hash)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            id,
            workspace_id,
            parent_id,
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
            deleted_at,
            created_at,
            updated_at
        "#,
        new_file.workspace_id,
        new_file.parent_id,
        new_file.file_type as FileType,
        new_file.name,
        new_file.path,
        new_file.hash
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(file)
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
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
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
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => Error::NotFound(format!("File not found: {}", id)),
        _ => Error::Sqlx(e),
    })?;

    Ok(file)
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
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
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

/// Updates the hash of a file and appends old hash to versions array.
/// This is the core versioning operation.
pub async fn update_file_hash(
    conn: &mut DbConn,
    file_id: Uuid,
    new_hash: &str,
    old_hash: Option<&str>,
) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        UPDATE files
        SET hash = $2,
            versions = CASE
                WHEN $3::text IS NOT NULL THEN array_append(versions, $3)
                ELSE versions
            END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            workspace_id,
            parent_id,
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
            deleted_at,
            created_at,
            updated_at
        "#,
        file_id,
        new_hash,
        old_hash
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(file)
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

/// Hard deletes a file from the database.
pub async fn hard_delete_file(conn: &mut DbConn, workspace_id: Uuid, file_id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        DELETE FROM files WHERE id = $1 AND workspace_id = $2
        "#,
        file_id,
        workspace_id
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("File not found or already deleted: {}", file_id)));
    }

    Ok(())
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
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
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

/// Checks if a file has any active (not deleted) children (via parent_id).
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

/// Checks if a path has any active (not deleted) descendants (via path prefix).
pub async fn has_active_descendants(conn: &mut DbConn, workspace_id: Uuid, path: &str) -> Result<bool> {
    let prefix = if path == "/" {
        "/".to_string()
    } else {
        format!("{}/", path.trim_end_matches('/'))
    };

    let result = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM files
            WHERE workspace_id = $1
              AND path LIKE $2 || '%'
              AND path != $3
              AND deleted_at IS NULL
        ) as "exists!"
        "#,
        workspace_id,
        prefix,
        path
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.exists)
}

/// Checks if a path collision exists in a target folder.
pub async fn check_path_collision(
    conn: &mut DbConn,
    workspace_id: Uuid,
    path: &str,
) -> Result<bool> {
    let result = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM files
            WHERE workspace_id = $1
              AND path = $2
              AND deleted_at IS NULL
        ) as "exists!"
        "#,
        workspace_id,
        path
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.exists)
}

/// Updates paths for all descendants of a folder.
pub async fn update_descendant_paths(
    conn: &mut DbConn,
    workspace_id: Uuid,
    old_path_prefix: &str,
    new_path_prefix: &str,
) -> Result<()> {
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
        old_path_prefix,
        old_prefix_slash
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

/// Updates file metadata (parent_id, name, path).
pub async fn update_file_metadata(
    conn: &mut DbConn,
    file_id: Uuid,
    parent_id: Option<Uuid>,
    name: &str,
    path: &str,
) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        UPDATE files
        SET parent_id = $2, name = $3, path = $4, updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            workspace_id,
            parent_id,
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
            deleted_at,
            created_at,
            updated_at
        "#,
        file_id,
        parent_id,
        name,
        path
    )
    .fetch_one(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(file)
}

/// Performs a soft delete on a file.
pub async fn soft_delete_file(conn: &mut DbConn, file_id: Uuid) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        UPDATE files
        SET deleted_at = NOW(), updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        file_id
    )
    .execute(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(result.rows_affected())
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
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
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
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
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

/// Gets all active (non-deleted) files in a workspace.
pub async fn list_all_active_files(conn: &mut DbConn, workspace_id: Uuid) -> Result<Vec<File>> {
    let files = sqlx::query_as!(
        File,
        r#"
        SELECT
            id,
            workspace_id,
            parent_id,
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
            deleted_at,
            created_at,
            updated_at
        FROM files
        WHERE workspace_id = $1
          AND deleted_at IS NULL
        ORDER BY path ASC
        "#,
        workspace_id
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(files)
}

/// Gets all active files of a specific type in a workspace.
pub async fn get_files_by_type(
    conn: &mut DbConn,
    workspace_id: Uuid,
    file_type: FileType,
) -> Result<Vec<File>> {
    let files = sqlx::query_as!(
        File,
        r#"
        SELECT
            id,
            workspace_id,
            parent_id,
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
            deleted_at,
            created_at,
            updated_at
        FROM files
        WHERE workspace_id = $1
          AND file_type = $2
          AND deleted_at IS NULL
        ORDER BY path ASC
        "#,
        workspace_id,
        file_type as FileType
    )
    .fetch_all(conn)
    .await
    .map_err(Error::Sqlx)?;

    Ok(files)
}

/// Restores a specific version by hash.
/// Copies the hash from versions array to current hash.
pub async fn restore_version(conn: &mut DbConn, file_id: Uuid, version_hash: &str) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        UPDATE files
        SET hash = $2,
            versions = array_append(versions, hash),
            updated_at = NOW()
        WHERE id = $1 AND $2 = ANY(versions)
        RETURNING
            id,
            workspace_id,
            parent_id,
            file_type as "file_type: FileType",
            name,
            path,
            hash,
            versions,
            deleted_at,
            created_at,
            updated_at
        "#,
        file_id,
        version_hash
    )
    .fetch_one(conn)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => Error::NotFound(format!("Version not found for file: {}", version_hash)),
        _ => Error::Sqlx(e),
    })?;

    Ok(file)
}

/// Gets all version hashes for a file (current + history).
pub async fn get_file_versions(conn: &mut DbConn, file_id: Uuid) -> Result<(Option<String>, Vec<String>)> {
    let file = get_file_by_id(conn, file_id).await?;
    Ok((file.hash, file.versions.unwrap_or_default()))
}
