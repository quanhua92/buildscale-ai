use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, LsArgs, LsResult, LsEntry}, queries::files};
use crate::services::storage::FileStorageService;
use crate::models::files::FileType;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use super::{Tool, ToolConfig};

/// List directory contents tool
///
/// Lists files and folders in a directory within a workspace.
/// Uses hybrid discovery: database listing + filesystem scan for external files.
///
/// # Discovery Strategy
///
/// This tool combines two sources:
/// 1. **Database listing**: Primary source for files created through the API
/// 2. **Filesystem scan**: Discovers files created externally (SSH, migrations, AI agents)
///
/// Database entries take precedence. Files only on disk are added with minimal metadata.
///
/// # Security: Workspace Isolation
///
/// This tool ensures workspace isolation by:
/// 1. Using workspace_id to get the correct storage path
/// 2. Validating path patterns to prevent traversal attacks
/// 3. Scanning only within the workspace directory
pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &'static str {
        "ls"
    }

    fn description(&self) -> &'static str {
        "Lists files and folders in a workspace directory. All parameters are optional. Returns entries sorted with folders first.

DISCOVERY STRATEGY:
- Hybrid approach: Database entries + filesystem scan for external files
- Database entries have full metadata (id, display_name, is_virtual, etc.)
- External files (created via SSH, migrations, etc.) have minimal metadata
- Files moved via mv tool are now visible (bugfix consistency)

Parameters:
- path (string, optional): workspace directory path. Default: '/' for workspace root.
- recursive (boolean, optional): list all subdirectories recursively. Default: false.

USAGE EXAMPLES:
- Good (list root): {}
- Good (list specific folder): {\"path\": \"/src\"}
- Good (recursive listing): {\"path\": \"/src\", \"recursive\": true}
- Good (explicit nulls): {\"path\": null, \"recursive\": null}

BAD EXAMPLES (will fail):
- Bad (string instead of object): \"/src\"
- Bad (array instead of object): [\"/src\"]
- Bad (extra properties): {\"path\": \"/src\", \"invalid\": true}"
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": ["string", "null"]},
                "recursive": {
                    "type": ["boolean", "string", "null"],
                    "description": "Accepts JSON boolean (true/false) or string representations ('true', 'True', 'false', 'False', 'TRUE', 'FALSE'). Defaults to false if not provided."
                }
            },
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        conn: &mut DbConn,
        storage: &FileStorageService,
        workspace_id: Uuid,
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let ls_args: LsArgs = serde_json::from_value(args)?;
        let path = super::normalize_path(&ls_args.path.unwrap_or_else(|| "/".to_string()));
        let recursive = ls_args.recursive.unwrap_or(false);

        let parent_id = if path == "/" {
            None
        } else {
            let parent_file = files::get_file_by_path(conn, workspace_id, &path)
                .await?
                .ok_or_else(|| Error::NotFound(format!("Directory not found: {}", path)))?;

            if !matches!(parent_file.file_type, FileType::Folder) {
                return Err(Error::Validation(crate::error::ValidationErrors::Single {
                    field: "path".to_string(),
                    message: format!("Path is not a directory: {}", path),
                }));
            }

            Some(parent_file.id)
        };

        // Phase 1: Get database listing (existing logic)
        let db_files = if recursive {
            Self::list_files_recursive(conn, workspace_id, &path).await?
        } else {
            files::list_files_in_folder(conn, workspace_id, parent_id).await?
        };

        // Phase 2: Get filesystem listing (NEW - hybrid discovery)
        let workspace_path = storage.get_workspace_path(workspace_id);
        let fs_entries = Self::list_filesystem_entries(&workspace_path, &path, recursive).await?;

        // Phase 3: Merge database + filesystem entries
        // Database entries take precedence, filesystem-only entries added as fallback
        let merged_entries = Self::merge_entries(db_files, fs_entries, &workspace_path).await?;

        let result = LsResult { path, entries: merged_entries };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}

impl LsTool {
    /// Lists files recursively using SQL path prefix matching
    async fn list_files_recursive(
        conn: &mut DbConn,
        workspace_id: Uuid,
        path_prefix: &str,
    ) -> Result<Vec<crate::models::files::File>> {
        let files = sqlx::query_as!(
            crate::models::files::File,
            r#"
            SELECT
                id, workspace_id, parent_id, author_id,
                file_type as "file_type: crate::models::files::FileType",
                status as "status: crate::models::files::FileStatus",
                name, slug, path,
                is_virtual, is_remote, permission,
                latest_version_id,
                deleted_at, created_at, updated_at
            FROM files
            WHERE workspace_id = $1
              AND path LIKE $2 || '%'
              AND path != $2
              AND deleted_at IS NULL
            ORDER BY path ASC
            "#,
            workspace_id,
            path_prefix
        )
        .fetch_all(conn)
        .await
        .map_err(Error::Sqlx)?;

        Ok(files)
    }

    /// Lists entries on the filesystem (hybrid discovery)
    ///
    /// Discovers files on disk that may not be in the database:
    /// - Files created via SSH
    /// - Files from migration scripts
    /// - Files moved by mv tool
    /// - Files from external tools
    ///
    /// Uses structured filesystem APIs (tokio::fs::read_dir) instead of parsing ls output.
    /// This avoids cross-platform parsing issues and provides reliable results.
    ///
    /// Uses iterative approach (not recursive) to avoid async recursion boxing.
    async fn list_filesystem_entries(
        workspace_path: &Path,
        ls_path: &str,
        recursive: bool,
    ) -> Result<Vec<FilesystemEntry>> {
        let mut entries = Vec::new();

        // Convert workspace path to filesystem path
        // ls_path "/" -> workspace_path
        // ls_path "/src" -> workspace_path/src
        let start_dir = if ls_path == "/" {
            workspace_path.to_path_buf()
        } else {
            // Strip leading slash and join with workspace path
            let relative_path = ls_path.strip_prefix('/').unwrap_or(ls_path);
            workspace_path.join(relative_path)
        };

        // Use iterative approach with a stack to avoid async recursion
        let mut dirs_to_scan = vec![(ls_path.to_string(), start_dir)];

        while let Some((current_ls_path, current_fs_path)) = dirs_to_scan.pop() {
            // Check if directory exists on disk
            if !current_fs_path.exists() {
                continue;
            }

            // Scan directory entries
            let mut read_dir = match tokio::fs::read_dir(&current_fs_path).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
                Error::Internal(format!("Failed to read directory entry: {}", e))
            })? {
                let file_type = entry.file_type().await.map_err(|e| {
                    Error::Internal(format!("Failed to get file type: {}", e))
                })?;

                // Skip symlinks and special files to avoid complexity
                if file_type.is_symlink() {
                    continue;
                }

                let name = entry.file_name().to_string_lossy().to_string();
                let full_path = entry.path();

                // Determine file type
                let file_type_enum = if file_type.is_dir() {
                    FileType::Folder
                } else {
                    FileType::Document
                };

                // Get file metadata (size, modified time)
                let metadata = entry.metadata().await.map_err(|e| {
                    Error::Internal(format!("Failed to get file metadata: {}", e))
                })?;

                let updated_at: chrono::DateTime<chrono::Utc> = metadata
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

                entries.push(FilesystemEntry {
                    name: name.clone(),
                    path: full_path,
                    file_type: file_type_enum,
                    updated_at,
                });

                // Add subdirectories for recursive scan
                if recursive && file_type.is_dir() {
                    let child_ls_path = if current_ls_path == "/" {
                        format!("/{}", name)
                    } else {
                        format!("{}/{}", current_ls_path, name)
                    };
                    dirs_to_scan.push((child_ls_path, entry.path()));
                }
            }
        }

        Ok(entries)
    }

    /// Merges database entries and filesystem entries
    ///
    /// Strategy:
    /// 1. Database entries take precedence (full metadata)
    /// 2. Filesystem-only entries added with minimal metadata
    /// 3. Deduplication by path
    async fn merge_entries(
        db_files: Vec<crate::models::files::File>,
        fs_entries: Vec<FilesystemEntry>,
        workspace_path: &Path,
    ) -> Result<Vec<LsEntry>> {
        use std::collections::{HashMap, HashSet};

        // Build lookup: path -> database file
        let mut db_lookup: HashMap<String, crate::models::files::File> = HashMap::new();
        for file in db_files {
            db_lookup.insert(file.path.clone(), file);
        }

        // Track which paths we've seen from database
        let mut db_paths: HashSet<String> = HashSet::new();

        let mut merged = Vec::new();

        // Add database entries first (they have full metadata)
        for file in db_lookup.values() {
            db_paths.insert(file.path.clone());
            merged.push(LsEntry {
                id: Some(file.id),
                synced: true,  // Database entry
                name: file.slug.clone(),
                display_name: file.name.clone(),
                path: file.path.clone(),
                file_type: file.file_type,
                is_virtual: file.is_virtual,
                updated_at: file.updated_at,
            });
        }

        // Add filesystem-only entries (not in database)
        for fs_entry in fs_entries {
            // Convert filesystem path to workspace path
            // /path/to/workspace/latest/src/file.rs -> /src/file.rs
            let workspace_relative = fs_entry
                .path
                .strip_prefix(workspace_path.join("latest"))
                .or_else(|_| fs_entry.path.strip_prefix(workspace_path))
                .unwrap_or(&fs_entry.path);

            let workspace_path_str = format!("/{}", workspace_relative.to_string_lossy());

            // Skip if already in database
            if db_paths.contains(&workspace_path_str) {
                continue;
            }

            // For filesystem-only files, use minimal metadata
            let name = fs_entry.name.clone();
            let display_name = name.clone();

            merged.push(LsEntry {
                id: None,  // No database ID for filesystem-only files
                synced: false,  // Filesystem-only
                name,
                display_name,
                path: workspace_path_str,
                file_type: fs_entry.file_type,
                is_virtual: false,
                updated_at: fs_entry.updated_at,
            });
        }

        // Sort: folders first, then by path
        merged.sort_by(|a, b| {
            match (&a.file_type, &b.file_type) {
                (FileType::Folder, FileType::Folder) => a.path.cmp(&b.path),
                (FileType::Folder, _) => std::cmp::Ordering::Less,
                (_, FileType::Folder) => std::cmp::Ordering::Greater,
                _ => a.path.cmp(&b.path),
            }
        });

        Ok(merged)
    }
}

/// Filesystem entry discovered during directory scan
///
/// Represents a file or folder found on disk during filesystem discovery.
/// Used to merge with database entries for hybrid listing.
#[derive(Debug, Clone)]
struct FilesystemEntry {
    name: String,
    path: PathBuf,
    file_type: FileType,
    updated_at: chrono::DateTime<chrono::Utc>,
}
