use chrono::{DateTime, Utc};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, sqlx::Type,
)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum FileType {
    Folder,
    Document,
    Canvas,
    Chat,
    Whiteboard,
    Agent,
    Skill,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, sqlx::Type,
)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum FileStatus {
    Pending,
    Uploading,
    Waiting,
    Processing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
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

    /// Cache for the latest version to avoid expensive JOINs/CTEs
    pub latest_version_id: Option<Uuid>,

    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFile {
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub author_id: Uuid,
    pub file_type: FileType,
    pub status: FileStatus,
    pub name: String,
    pub slug: String,
    pub path: String,
    pub is_virtual: bool,
    pub is_remote: bool,
    pub permission: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub id: Uuid,
    pub file_id: Uuid,
    pub workspace_id: Uuid,
    pub branch: String,
    pub app_data: serde_json::Value,
    pub hash: String,
    pub author_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFileVersion {
    pub id: Option<Uuid>,
    pub file_id: Uuid,
    pub workspace_id: Uuid,
    pub branch: String,
    pub app_data: serde_json::Value,
    pub hash: String,
    pub author_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub chunk_hash: String,
    pub chunk_content: String,
    pub embedding: Option<Vector>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLink {
    pub source_file_id: Uuid,
    pub target_file_id: Uuid,
    pub workspace_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTag {
    pub file_id: Uuid,
    pub workspace_id: Uuid,
    pub tag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_file_type_serialization() {
        let ft = FileType::Folder;
        assert_eq!(ft.to_string(), "folder");
        assert_eq!(FileType::from_str("folder").unwrap(), FileType::Folder);
    }

    #[test]
    fn test_file_status_serialization() {
        let fs = FileStatus::Ready;
        assert_eq!(fs.to_string(), "ready");
        assert_eq!(FileStatus::from_str("ready").unwrap(), FileStatus::Ready);
    }
}
