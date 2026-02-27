use chrono::{DateTime, Utc};
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
    Plan,
    Memory,
}

/// Simplified File model (Obsidian-style)
/// - Single table with hash + versions array
/// - No separate version table
/// - Archive provides backup copies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub file_type: FileType,
    pub name: String,
    pub path: String,
    /// SHA-256 of current content (nullable for folders)
    pub hash: Option<String>,
    /// Array of historical hashes (for history traversal)
    #[serde(default)]
    pub versions: Option<Vec<String>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFile {
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub file_type: FileType,
    pub name: String,
    pub path: String,
    /// Initial hash (None for folders)
    pub hash: Option<String>,
}

/// Request to update file content (creates new version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFileContent {
    pub file_id: Uuid,
    pub new_hash: String,
    /// Old hash to archive
    pub old_hash: Option<String>,
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
}
