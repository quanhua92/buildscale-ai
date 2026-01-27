use crate::models::{
    files::{File, FileType, FileVersion},
    roles::Role,
    workspace_members::WorkspaceMember,
    workspaces::Workspace,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request for creating a workspace with automatic setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub owner_id: Uuid,
}

/// HTTP API request for creating a workspace (owner_id extracted from JWT)
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkspaceHttp {
    pub name: String,
}

/// Request for creating a workspace with initial members
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceWithMembersRequest {
    pub name: String,
    pub owner_id: Uuid,
    pub members: Vec<WorkspaceMemberRequest>,
}

/// Request for adding a member to a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMemberRequest {
    pub user_id: Uuid,
    pub role_name: String, // Use role name for convenience (admin, editor, viewer)
}

/// Request for user registration with workspace creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWorkspaceRegistrationRequest {
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub full_name: Option<String>,
    pub workspace_name: String,
}

/// Result of a complete workspace creation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteWorkspaceResult {
    pub workspace: Workspace,
    pub roles: Vec<Role>,
    pub owner_membership: WorkspaceMember,
    pub members: Vec<WorkspaceMember>,
}

/// Result of user registration with workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWorkspaceResult {
    pub user: super::users::User,
    pub workspace: CompleteWorkspaceResult,
}

/// Request to update workspace details
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: String,
}

/// Request for creating a new file with initial content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFileRequest {
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub author_id: Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub path: Option<String>,
    pub is_virtual: Option<bool>,
    pub is_remote: Option<bool>,
    pub permission: Option<i32>,
    pub file_type: FileType,
    pub content: serde_json::Value,
    pub app_data: Option<serde_json::Value>,
}

/// Request for creating a new version of an existing file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    pub author_id: Option<Uuid>,
    pub branch: Option<String>,
    pub content: serde_json::Value,
    pub app_data: Option<serde_json::Value>,
}

/// HTTP API request for creating a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFileHttp {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub slug: Option<String>,
    pub path: Option<String>,
    pub is_virtual: Option<bool>,
    pub is_remote: Option<bool>,
    pub permission: Option<i32>,
    pub file_type: FileType,
    pub content: serde_json::Value,
    pub app_data: Option<serde_json::Value>,
}

/// HTTP API request for creating a new version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionHttp {
    pub branch: Option<String>,
    pub content: serde_json::Value,
    pub app_data: Option<serde_json::Value>,
}

/// Combined model for a file and its latest content version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWithContent {
    pub file: File,
    pub latest_version: FileVersion,
}

/// HTTP API request for updating file metadata (move/rename)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateFileHttp {
    /// New parent folder.
    /// - `None`: Field not present, do not change.
    /// - `Some(None)`: Move to root.
    /// - `Some(Some(uuid))`: Move to folder.
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub parent_id: Option<Option<Uuid>>,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub is_virtual: Option<bool>,
    pub is_remote: Option<bool>,
    pub permission: Option<i32>,
}

/// Service request for updating file metadata
#[derive(Debug, Clone, Default)]
pub struct UpdateFileRequest {
    pub parent_id: Option<Option<Uuid>>,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub is_virtual: Option<bool>,
    pub is_remote: Option<bool>,
    pub permission: Option<i32>,
}

/// Helper to deserialize double options (None = missing, Some(None) = null, Some(Some) = value)
fn deserialize_double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// HTTP API request for adding a tag to a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTagHttp {
    pub tag: String,
}

/// HTTP API request for creating a link between files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLinkHttp {
    pub target_file_id: Uuid,
}

/// Summary of a file's network relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNetworkSummary {
    pub tags: Vec<String>,
    pub outbound_links: Vec<File>,
    pub backlinks: Vec<File>,
}

/// Request for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchHttp {
    pub query_vector: Vec<f32>,
    pub limit: Option<i32>,
}

/// Single result from a semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: File,
    pub chunk_content: String,
    pub similarity: f32,
}

// ============================================================================
// TOOL REQUEST AND RESPONSE MODELS
// ============================================================================

/// Unified tool request structure
#[derive(Debug, Clone, Deserialize)]
pub struct ToolRequest {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateChatRequest {
    pub goal: String,
    pub files: Option<Vec<Uuid>>,
    pub agents: Option<Vec<Uuid>>,
    pub model: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostChatMessageRequest {
    pub content: String,
    pub model: Option<String>,
}

/// Tool-specific argument structures
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LsArgs {
    pub path: Option<String>,
    pub recursive: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadArgs {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteArgs {
    pub path: String,
    pub content: serde_json::Value,
    pub file_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RmArgs {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MvArgs {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MkdirArgs {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EditArgs {
    pub path: String,
    pub old_string: String,
    pub new_string: String,
    pub last_read_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GrepArgs {
    pub pattern: String,
    pub path_pattern: Option<String>,
    pub case_sensitive: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TouchArgs {
    pub path: String,
}

/// Unified tool response structure
#[derive(Debug, Clone, Serialize)]
pub struct ToolResponse {
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

/// Tool-specific result structures
#[derive(Debug, Clone, Serialize)]
pub struct LsResult {
    pub path: String,
    pub entries: Vec<LsEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LsEntry {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub path: String,
    pub file_type: FileType,
    pub is_virtual: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrepResult {
    pub matches: Vec<GrepMatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrepMatch {
    pub path: String,
    pub line_number: i32,
    pub line_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadResult {
    pub path: String,
    pub content: serde_json::Value,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteResult {
    pub path: String,
    pub file_id: Uuid,
    pub version_id: Uuid,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RmResult {
    pub path: String,
    pub file_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct MvResult {
    pub from_path: String,
    pub to_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MkdirResult {
    pub path: String,
    pub file_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TouchResult {
    pub path: String,
    pub file_id: Uuid,
}
