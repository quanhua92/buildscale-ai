use crate::error::{Error, Result};
use crate::services::files::slugify;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

pub struct FileStorageService {
    base_path: PathBuf,
}

impl FileStorageService {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: PathBuf::from(base_path),
        }
    }

    /// Initializes storage directory structure
    pub async fn init(&self) -> Result<()> {
        // Create base workspaces directory
        let workspaces_dir = self.get_workspaces_root();
        if !workspaces_dir.exists() {
            fs::create_dir_all(&workspaces_dir).await.map_err(|e| {
                Error::Internal(format!("Failed to create workspaces directory {:?}: {}", workspaces_dir, e))
            })?;
        }
        Ok(())
    }

    // --- Path Helpers ---

    fn get_workspaces_root(&self) -> PathBuf {
        self.base_path.join("workspaces")
    }

    fn get_workspace_root(&self, workspace_id: Uuid) -> PathBuf {
        self.get_workspaces_root().join(workspace_id.to_string())
    }

    pub fn get_workspace_path(&self, workspace_id: Uuid) -> PathBuf {
        self.get_latest_root(workspace_id)
    }

    fn get_latest_root(&self, workspace_id: Uuid) -> PathBuf {
        self.get_workspace_root(workspace_id).join("latest")
    }

    fn get_archive_root(&self, workspace_id: Uuid) -> PathBuf {
        self.get_workspace_root(workspace_id).join("archive")
    }

    fn get_trash_root(&self, workspace_id: Uuid) -> PathBuf {
        self.get_workspace_root(workspace_id).join("trash")
    }

    fn get_file_path(&self, workspace_id: Uuid, path: &str) -> Result<PathBuf> {
        // Validate path components to prevent traversal
        let path_obj = std::path::Path::new(path);

        // Reject parent directory references (..) which could escape workspace
        if path_obj.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "path".to_string(),
                message: "Path cannot contain '..' (parent directory references)".to_string(),
            }));
        }

        // Normalize (strip leading slash for consistency)
        let clean_path = path.trim_start_matches('/');

        Ok(self.get_latest_root(workspace_id).join(clean_path))
    }

    fn get_archive_path(&self, workspace_id: Uuid, hash: &str) -> PathBuf {
        // Use 2-level directory sharding for archive (e.g. /archive/e3/b0/...)
        // to prevent too many files in one directory
        if hash.len() < 4 {
            return self.get_archive_root(workspace_id).join(hash);
        }
        let l1 = &hash[0..2];
        let l2 = &hash[2..4];
        self.get_archive_root(workspace_id).join(l1).join(l2).join(hash)
    }

    // --- Core Operations ---

    /// Reads a file from Latest directory (O(1) access)
    pub async fn read_file(&self, workspace_id: Uuid, path: &str) -> Result<Vec<u8>> {
        let full_path = self.get_file_path(workspace_id, path)?;

        if !full_path.exists() {
            return Err(Error::NotFound(format!("File not found on disk: {}", path)));
        }

        fs::read(&full_path).await.map_err(|e| {
            Error::Internal(format!("Failed to read file {:?}: {}", full_path, e))
        })
    }

    /// Reads a specific version from Archive
    pub async fn read_version(&self, workspace_id: Uuid, hash: &str) -> Result<Vec<u8>> {
        let full_path = self.get_archive_path(workspace_id, hash);

        if !full_path.exists() {
            return Err(Error::NotFound(format!("Version blob not found: {}", hash)));
        }

        fs::read(&full_path).await.map_err(|e| {
            Error::Internal(format!("Failed to read version {:?}: {}", full_path, e))
        })
    }

    /// Writes content only to the Latest directory (Working Tree).
    /// Used for "Healing" the working tree from the archive.
    pub async fn write_latest_file(&self, workspace_id: Uuid, path: &str, content: &[u8]) -> Result<()> {
        let file_path = self.get_file_path(workspace_id, path)?;

        // Ensure parent directories exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                Error::Internal(format!("Failed to create directory {:?}: {}", parent, e))
            })?;
        }

        fs::write(&file_path, content).await.map_err(|e| {
            Error::Internal(format!("Failed to write working file {:?}: {}", file_path, e))
        })?;

        Ok(())
    }

    /// Writes content with a pre-calculated hash.
    /// This allows salting the hash (e.g. with version_id) for safer deduplication.
    pub async fn write_file_with_hash(
        &self,
        workspace_id: Uuid,
        path: &str,
        content: &[u8],
        hash: &str,
    ) -> Result<()> {
        // 1. Archive (Version-Unique)
        let archive_path = self.get_archive_path(workspace_id, hash);
        if !archive_path.exists() {
            if let Some(parent) = archive_path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    Error::Internal(format!("Failed to create archive directory {:?}: {}", parent, e))
                })?;
            }
            fs::write(&archive_path, content).await.map_err(|e| {
                Error::Internal(format!("Failed to write archive blob {:?}: {}", archive_path, e))
            })?;
        }

        // 2. Latest directory
        self.write_latest_file(workspace_id, path, content).await?;

        Ok(())
    }

    /// Creates a directory for a folder
    pub async fn create_folder(&self, workspace_id: Uuid, path: &str) -> Result<()> {
        let dir_path = self.get_file_path(workspace_id, path)?;

        // Create the folder directory (idempotent - won't fail if already exists)
        fs::create_dir_all(&dir_path).await.map_err(|e| {
            Error::Internal(format!("Failed to create folder directory {:?}: {}", dir_path, e))
        })?;

        Ok(())
    }

    /// Appends content to a file (Used for Chat Logs)
    /// Note: This bypasses Archive for performance (Archive is for snapshots/versions).
    /// To "Version" a chat log, a full snapshot should be triggered separately.
    pub async fn append_to_file(&self, workspace_id: Uuid, path: &str, content: &str) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let file_path = self.get_file_path(workspace_id, path)?;

        // Ensure parent directories exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                Error::Internal(format!("Failed to create directory {:?}: {}", parent, e))
            })?;
        }

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await
            .map_err(|e| Error::Internal(format!("Failed to open file for append {:?}: {}", file_path, e)))?;

        file.write_all(content.as_bytes()).await.map_err(|e| {
            Error::Internal(format!("Failed to append to file {:?}: {}", file_path, e))
        })?;

        Ok(())
    }

    /// Soft deletes a file by moving it to Trash
    pub async fn move_to_trash(&self, workspace_id: Uuid, path: &str) -> Result<()> {
        let source_path = self.get_file_path(workspace_id, path)?;

        if !source_path.exists() {
            // If file doesn't exist on disk, we just ignore it (db metadata might be out of sync,
            // but goal "file is gone" is met)
            return Ok(());
        }

        // Create trash path: trash/<workspace_id>/<timestamp>_<slugified_path>
        let timestamp = chrono::Utc::now().timestamp();
        let safe_path_name = slugify(path);
        let trash_filename = format!("{}_{}", timestamp, safe_path_name);

        let trash_dir = self.get_trash_root(workspace_id);
        if !trash_dir.exists() {
            fs::create_dir_all(&trash_dir).await.map_err(|e| {
                Error::Internal(format!("Failed to create trash directory {:?}: {}", trash_dir, e))
            })?;
        }

        let trash_path = trash_dir.join(trash_filename);

        fs::rename(&source_path, &trash_path).await.map_err(|e| {
            Error::Internal(format!("Failed to move file to trash {:?} -> {:?}: {}", source_path, trash_path, e))
        })?;

        Ok(())
    }

    /// Restores a file from Trash or Archive
    /// If file exists in Latest, does nothing.
    /// Otherwise, attempts to restore from Archive using the provided hash.
    pub async fn ensure_file_restored(&self, workspace_id: Uuid, path: &str, hash: &str) -> Result<()> {
        let target_path = self.get_file_path(workspace_id, path)?;

        if target_path.exists() {
            return Ok(());
        }

        // Restore from archive
        let archive_path = self.get_archive_path(workspace_id, hash);
        if !archive_path.exists() {
             return Err(Error::Internal(format!("Cannot restore file: Archive blob missing for hash {}", hash)));
        }

        // Ensure parent directories exist
        if let Some(parent) = target_path.parent() {
             fs::create_dir_all(parent).await.map_err(|e| {
                 Error::Internal(format!("Failed to create directory {:?}: {}", parent, e))
             })?;
        }

        fs::copy(&archive_path, &target_path).await.map_err(|e| {
            Error::Internal(format!("Failed to restore file from archive: {}", e))
        })?;

        Ok(())
    }

    /// Deletes a specific version blob from Archive
    pub async fn delete_archive_blob(&self, workspace_id: Uuid, hash: &str) -> Result<()> {
        let full_path = self.get_archive_path(workspace_id, hash);

        if !full_path.exists() {
            return Ok(()); // Already gone
        }

        fs::remove_file(&full_path).await.map_err(|e| {
            Error::Internal(format!("Failed to delete archive blob {:?}: {}", full_path, e))
        })?;

        Ok(())
    }

    /// Handles rename/move operations on disk
    pub async fn move_file(&self, workspace_id: Uuid, old_path: &str, new_path: &str) -> Result<()> {
        let source = self.get_file_path(workspace_id, old_path)?;
        let target = self.get_file_path(workspace_id, new_path)?;

        if !source.exists() {
             return Err(Error::NotFound(format!("File not found for move: {}", old_path)));
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                Error::Internal(format!("Failed to create directory {:?}: {}", parent, e))
            })?;
        }

        fs::rename(&source, &target).await.map_err(|e| {
             Error::Internal(format!("Failed to move file {:?} -> {:?}: {}", source, target, e))
        })?;

        Ok(())
    }
}
