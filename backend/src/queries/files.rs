use crate::{
    error::{Error, Result},
    models::files::{File, NewFile, FileType, FileStatus},
    DbConn,
};
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
