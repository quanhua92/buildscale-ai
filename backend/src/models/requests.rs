use crate::models::{
    files::{File, FileType, FileVersion},
    roles::Role,
    workspace_members::WorkspaceMember,
    workspaces::Workspace,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Wrapper for JSON values with OpenAI-compatible schema
///
/// Accepts any JSON value (string, number, boolean, array, object).
/// The AI should pass JSON as a string, which gets parsed into the actual value.
#[derive(Debug, Clone)]
pub struct JsonValue(pub serde_json::Value);

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        // If the value is a string, try to parse it as JSON
        // This handles cases where the LLM provides a JSON object as a stringified JSON
        if let serde_json::Value::String(s) = &value {
            if let Ok(parsed_value) = serde_json::from_str(s) {
                return Ok(JsonValue(parsed_value));
            }
        }
        Ok(JsonValue(value))
    }
}

impl Serialize for JsonValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl From<JsonValue> for serde_json::Value {
    fn from(val: JsonValue) -> Self {
        val.0
    }
}

impl From<serde_json::Value> for JsonValue {
    fn from(val: serde_json::Value) -> Self {
        JsonValue(val)
    }
}

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
    pub content: serde_json::Value,
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

/// Custom deserializer for flexible boolean parsing
/// Accepts:
/// - JSON booleans: true, false
/// - Strings (case-insensitive): "true", "True", "TRUE", "false", "False", "FALSE"
/// - Numbers: 1 (true), 0 (false)
pub fn deserialize_flexible_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    match serde_json::Value::deserialize(deserializer)? {
        // Accept JSON boolean directly
        serde_json::Value::Bool(b) => Ok(b),

        // Accept string representations (case-insensitive)
        serde_json::Value::String(s) => {
            match s.to_lowercase().as_str() {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                _ => Err(D::Error::custom(format!(
                    "Invalid boolean string: '{}'. Expected: true, false, 1, or 0",
                    s
                ))),
            }
        },

        // Accept integers (1 = true, 0 = false)
        serde_json::Value::Number(n) => {
            if let Some(b) = n.as_u64() {
                match b {
                    1 => Ok(true),
                    0 => Ok(false),
                    _ => Err(D::Error::custom(format!(
                        "Invalid boolean number: '{}'. Expected: 0 or 1",
                        b
                    ))),
                }
            } else {
                Err(D::Error::custom("Invalid boolean number".to_string()))
            }
        },

        other => Err(D::Error::custom(format!(
            "Invalid boolean type: {:?}. Expected: boolean, string, or number",
            other
        ))),
    }
}

/// Custom deserializer for flexible optional boolean parsing
/// Same as deserialize_flexible_bool but handles Option<bool>
pub fn deserialize_flexible_bool_option<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    match serde_json::Value::deserialize(deserializer)? {
        // Accept null directly
        serde_json::Value::Null => Ok(None),

        // Accept JSON boolean directly
        serde_json::Value::Bool(b) => Ok(Some(b)),

        // Accept string representations (case-insensitive)
        serde_json::Value::String(s) => {
            match s.to_lowercase().as_str() {
                "true" | "1" => Ok(Some(true)),
                "false" | "0" => Ok(Some(false)),
                _ => Err(D::Error::custom(format!(
                    "Invalid boolean string: '{}'. Expected: true, false, 1, 0, or null",
                    s
                ))),
            }
        },

        // Accept integers (1 = true, 0 = false)
        serde_json::Value::Number(n) => {
            if let Some(b) = n.as_u64() {
                match b {
                    1 => Ok(Some(true)),
                    0 => Ok(Some(false)),
                    _ => Err(D::Error::custom(format!(
                        "Invalid boolean number: '{}'. Expected: 0, 1, or null",
                        b
                    ))),
                }
            } else {
                Err(D::Error::custom("Invalid boolean number".to_string()))
            }
        },

        other => Err(D::Error::custom(format!(
            "Invalid boolean type: {:?}. Expected: boolean, string, number, or null",
            other
        ))),
    }
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
    /// Optional mode override (default: false = build mode)
    /// - false: Build mode - full tool access (write, edit, rm, mv, etc.)
    /// - true: Plan mode - restricted to plan files only
    #[serde(default, deserialize_with = "deserialize_flexible_bool")]
    pub plan_mode: bool,
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
    /// Optional metadata for the message (e.g., question answers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChatRequest {
    /// Application data to update (mode, plan_file, etc.)
    pub app_data: serde_json::Value,
}

/// Tool-specific argument structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsArgs {
    pub path: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_bool_option")]
    pub recursive: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadArgs {
    pub path: String,

    /// Optional starting line offset (0-indexed)
    /// Positive: from beginning (e.g., 100 = start at line 100)
    /// Negative: from end (e.g., -100 = last 100 lines)
    /// Default: 0 (read from beginning)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<isize>,

    /// Optional maximum number of lines to read
    /// Default: 500 (matches DEFAULT_READ_LIMIT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Optional cursor position for scroll mode (0-indexed line number)
    /// When set, enables scroll mode where offset is relative to cursor
    /// Example: cursor=100, offset=-50 reads lines 50-100 (scroll up 50 from cursor)
    /// Default: null (disabled, uses absolute offset mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteArgs {
    pub path: String,
    pub content: JsonValue,
    pub file_type: Option<String>,
    /// If false (default), returns error when file exists to prevent accidental overwrites.
    /// Set to true to explicitly overwrite existing files.
    /// Recommendation: Use 'edit' tool for modifying existing files instead of overwriting.
    #[serde(default, deserialize_with = "deserialize_flexible_bool")]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RmArgs {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MvArgs {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MkdirArgs {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditArgs {
    pub path: String,

    // For Replace operation: old_string and new_string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_string: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_string: Option<String>,

    // For Insert operation: insert_line and content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_line: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_read_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepArgs {
    pub pattern: String,
    pub path_pattern: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_bool_option")]
    pub case_sensitive: Option<bool>,
    /// Number of lines to show before each match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_context: Option<usize>,
    /// Number of lines to show after each match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_context: Option<usize>,
    /// Number of lines to show before and after each match (shorthand for before_context + after_context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchArgs {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserArgs {
    /// Array of questions (always array, single = 1-item array)
    pub questions: Vec<QuestionInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionInput {
    /// Question identifier (used in answer object)
    pub name: String,
    /// Question text (Markdown)
    pub question: String,
    /// JSON Schema for answer validation and UI generation
    pub schema: JsonValue,
    /// Optional button definitions (overrides schema-based rendering)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<QuestionButtonInput>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionButtonInput {
    pub label: String,
    /// Button value
    pub value: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitPlanModeArgs {
    pub plan_file_path: String,
}

// ============================================================================
// NEW TOOL ARGS AND RESULTS (Phase 1: glob, file_info, grep context)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobArgs {
    pub pattern: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobResult {
    pub pattern: String,
    pub base_path: String,
    pub matches: Vec<GlobMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobMatch {
    pub path: String,
    pub name: String,
    pub file_type: FileType,
    pub is_virtual: bool,
    pub size: Option<usize>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfoArgs {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileInfoResult {
    pub path: String,
    pub file_type: FileType,
    pub size: Option<usize>,
    pub line_count: Option<usize>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub hash: String,
}

// ============================================================================
// PHASE 2: read_multiple_files, edit insert, read scroll
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadMultipleFilesArgs {
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadMultipleFilesResult {
    pub files: Vec<ReadFileResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadFileResult {
    pub path: String,
    pub success: bool,
    pub content: Option<serde_json::Value>,
    pub hash: Option<String>,
    pub error: Option<String>,
    pub total_lines: Option<usize>,
    pub truncated: Option<bool>,
}

// ============================================================================
// PHASE 3: find, cat
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindArgs {
    pub name: Option<String>,
    pub path: Option<String>,
    pub file_type: Option<FileType>,
    pub min_size: Option<usize>,
    pub max_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindResult {
    pub matches: Vec<FindMatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindMatch {
    pub path: String,
    pub name: String,
    pub file_type: FileType,
    pub size: Option<usize>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatArgs {
    pub paths: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_bool_option")]
    pub show_headers: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_flexible_bool_option")]
    pub number_lines: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatResult {
    pub content: String,
    pub files: Vec<CatFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatFileEntry {
    pub path: String,
    pub content: String,
    pub line_count: usize,
}

/// Unified tool response structure
#[derive(Debug, Clone, Serialize)]
pub struct ToolResponse {
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

/// Tool-specific result structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsResult {
    pub path: String,
    pub entries: Vec<LsEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsEntry {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub path: String,
    pub file_type: FileType,
    pub is_virtual: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepResult {
    pub matches: Vec<GrepMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    pub path: String,
    pub line_number: i32,
    pub line_text: String,
    /// Context lines before the match (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_context: Option<Vec<String>>,
    /// Context lines after the match (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_context: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadResult {
    pub path: String,
    pub content: serde_json::Value,
    pub hash: String,

    /// Total number of lines in the file (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_lines: Option<usize>,

    /// Whether the content was truncated (partial read)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,

    /// The offset used for this read (actual start position, never negative)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,

    /// The limit used for this read
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Current cursor position (line number at end of read)
    /// Used for scroll mode to track position in large files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub path: String,
    pub file_id: Uuid,
    pub version_id: Uuid,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RmResult {
    pub path: String,
    pub file_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MvResult {
    pub from_path: String,
    pub to_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MkdirResult {
    pub path: String,
    pub file_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchResult {
    pub path: String,
    pub file_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct AskUserResult {
    pub status: String,
    pub question_id: Uuid,
    pub questions: Vec<crate::models::sse::Question>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExitPlanModeResult {
    pub mode: String,
    pub plan_file: String,
}
