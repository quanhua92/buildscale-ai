use crate::models::chat::{ChatMessage, ChatMessageMetadata, ChatMessageRole};
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

// --- Tool Result Truncation Constants ---

/// Number of recent tool results to keep full outputs for.
/// Older tool results are truncated to reduce context size since the AI can re-run tools.
pub const KEEP_RECENT_TOOL_RESULTS: usize = 5;

/// Maximum characters for truncated old tool results
pub const TRUNCATED_TOOL_RESULT_PREVIEW: usize = 50;

/// Identify which tool result indices should be truncated based on age.
///
/// Returns a HashSet of indices that are tool results and should be truncated
/// because they are older than KEEP_RECENT_TOOL_RESULTS.
///
/// # Arguments
/// * `tool_result_indices` - Slice of indices (in chronological order) that are tool results
///
/// # Returns
/// HashSet of indices to truncate
pub fn get_old_tool_result_indices(tool_result_indices: &[usize]) -> HashSet<usize> {
    let truncate_from_index = tool_result_indices.len().saturating_sub(KEEP_RECENT_TOOL_RESULTS);
    tool_result_indices
        .iter()
        .take(truncate_from_index)
        .copied()
        .collect()
}

/// Identify tool result indices in a sorted list of context items.
///
/// This is used by both AI context building and Context UI to consistently
/// identify which items are tool results.
///
/// # Arguments
/// * `items` - Slice of context items (should be sorted by timestamp)
///
/// # Returns
/// Vector of indices where tool result messages appear
pub fn get_tool_result_indices(items: &[ContextItem]) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            if let ContextItem::Message { metadata, .. } = item {
                if metadata.message_type.as_deref() == Some("tool_result") {
                    return Some(i);
                }
            }
            None
        })
        .collect()
}

/// Get indices of context items that should be truncated.
///
/// Combines tool result identification with age-based truncation logic.
///
/// # Arguments
/// * `items` - Slice of context items (should be sorted by timestamp)
///
/// # Returns
/// HashSet of indices that should be truncated
pub fn get_indices_to_truncate(items: &[ContextItem]) -> HashSet<usize> {
    let tool_result_indices = get_tool_result_indices(items);
    get_old_tool_result_indices(&tool_result_indices)
}

/// Truncate a string at a valid UTF-8 character boundary.
///
/// This function ensures we don't slice in the middle of a multi-byte character
/// (e.g., emoji like ✅ which is 3 bytes).
///
/// # Arguments
/// * `s` - The string to potentially truncate
/// * `max_bytes` - Maximum byte length (not character length)
///
/// # Returns
/// The byte index where truncation should occur (always at a char boundary)
pub fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> usize {
    if s.len() <= max_bytes {
        return s.len();
    }

    s.char_indices()
        .take_while(|(idx, _)| *idx < max_bytes)
        .last()
        .map(|(idx, c)| idx + c.len_utf8())
        .unwrap_or(0)
}

/// Truncate a tool result output if it's too long.
///
/// # Arguments
/// * `output` - The tool output string
///
/// # Returns
/// Truncated string with hint to re-run tool
pub fn truncate_tool_output(output: &str) -> String {
    if output.len() > TRUNCATED_TOOL_RESULT_PREVIEW {
        let truncate_at = truncate_at_char_boundary(output, TRUNCATED_TOOL_RESULT_PREVIEW);
        format!("{}…[re-run]", &output[..truncate_at])
    } else {
        output.to_string()
    }
}

/// Attachment key types for identifying different attachment sources.
///
/// Currently only `WorkspaceFile` is actively used. Other types are reserved
/// for future enhancements (skills, environment context, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "lowercase")]
pub enum AttachmentKey {
    SystemPersona,
    ActiveSkill(Uuid),
    WorkspaceFile(Uuid),
    Environment,
    ChatHistory,
    UserRequest,
}

/// Value structure for attachment fragments with metadata for pruning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentValue {
    pub content: String,
    pub priority: i32,
    pub tokens: usize,
    pub is_essential: bool,
    /// When the attachment was added to the context
    pub created_at: DateTime<Utc>,
    /// When the source content was last modified (e.g., file updated_at)
    pub updated_at: Option<DateTime<Utc>>,
}

/// Unified context item for chronological sorting of messages and attachments.
///
/// This enables cache-optimized context construction where older content
/// (both messages and attachments) becomes part of a stable, cacheable prefix.
#[derive(Debug, Clone)]
pub enum ContextItem {
    /// A chat message from the conversation history
    Message {
        role: ChatMessageRole,
        content: String,
        created_at: DateTime<Utc>,
        metadata: ChatMessageMetadata,
    },
    /// An attachment (file, skill, etc.) with rendered XML content
    Attachment {
        key: AttachmentKey,
        value: AttachmentValue,
        rendered: String,
    },
}

impl ContextItem {
    /// Get the timestamp for chronological sorting
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            ContextItem::Message { created_at, .. } => *created_at,
            ContextItem::Attachment { value, .. } => value.created_at,
        }
    }
}

pub type AttachmentMap = IndexMap<AttachmentKey, AttachmentValue>;

// --- Attachment Engineering Constants ---

// Positional Order (0 = Top of prompt, higher = later)
pub const POS_SYSTEM_PERSONA: i32 = 0;
pub const POS_SKILLS: i32 = 1;
pub const POS_WORKSPACE_FILES: i32 = 2;
pub const POS_ENVIRONMENT: i32 = 3;
pub const POS_CHAT_HISTORY: i32 = 4;
pub const POS_USER_REQUEST: i32 = 5;

// Pruning Priorities (Higher = dropped first during context pressure)
pub const PRIORITY_ESSENTIAL: i32 = 0;
pub const PRIORITY_HIGH: i32 = 3;
pub const PRIORITY_MEDIUM: i32 = 5;
pub const PRIORITY_LOW: i32 = 10;

// Token Estimation
pub const ESTIMATED_CHARS_PER_TOKEN: usize = 4;

/// Manages file attachments with priority-based pruning and token estimation.
///
/// # Purpose
///
/// `AttachmentManager` handles workspace file attachments for AI chat sessions:
/// - **Token Estimation**: Automatic counting using `ESTIMATED_CHARS_PER_TOKEN`
/// - **Priority-Based Pruning**: Drops low-priority files when over token limits
/// - **Keyed Addressability**: Access files by `WorkspaceFile(file_id)` key
/// - **XML Rendering**: Formats attachments with `<file_context>` markers
///
/// # Future Enhancements
///
/// Could be extended to manage:
/// - **HistoryManager**: For conversation history pruning and summarization
/// - **Skill Definitions**: Tool schemas attached to conversations
/// - **Environment Context**: CWD, git branch, OS info
///
/// # Example
///
/// ```text,no_run
/// use buildscale::services::chat::{AttachmentManager, AttachmentKey, AttachmentValue, PRIORITY_MEDIUM};
/// use uuid::Uuid;
///
/// let mut manager = AttachmentManager::new();
/// let file_id = Uuid::now_v7();
///
/// // Add a file attachment with estimated tokens
/// manager.add_fragment(
///     AttachmentKey::WorkspaceFile(file_id),
///     AttachmentValue {
///         content: "File content here".to_string(),
///         priority: PRIORITY_MEDIUM,
///         tokens: 100,  // Estimated token count
///         is_essential: false,
///     },
/// );
///
/// // Optimize for token limit
/// manager.optimize_for_limit(4000);
///
/// // Render with XML markers
/// let rendered = manager.render();
/// ```
#[derive(Debug, Clone)]
pub struct AttachmentManager {
    pub map: AttachmentMap,
}

impl Default for AttachmentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AttachmentManager {
    pub fn new() -> Self {
        Self {
            map: IndexMap::new(),
        }
    }

    pub fn add_fragment(&mut self, key: AttachmentKey, value: AttachmentValue) {
        self.map.insert(key, value);
    }

    /// Sorts fragments based on their natural logical order:
    /// System -> Skills -> Files -> Env -> History -> Request
    pub fn sort_by_position(&mut self) {
        self.map.sort_by(|ka, _, kb, _| {
            let pos_a = Self::get_key_position(ka);
            let pos_b = Self::get_key_position(kb);
            pos_a.cmp(&pos_b)
        });
    }

    fn get_key_position(key: &AttachmentKey) -> i32 {
        match key {
            AttachmentKey::SystemPersona => POS_SYSTEM_PERSONA,
            AttachmentKey::ActiveSkill(_) => POS_SKILLS,
            AttachmentKey::WorkspaceFile(_) => POS_WORKSPACE_FILES,
            AttachmentKey::Environment => POS_ENVIRONMENT,
            AttachmentKey::ChatHistory => POS_CHAT_HISTORY,
            AttachmentKey::UserRequest => POS_USER_REQUEST,
        }
    }

    /// Prunes attachments to fit within a token limit.
    ///
    /// Removes fragments with the highest priority values (lowest importance)
    /// that are not marked as essential.
    ///
    /// # Algorithm
    ///
    /// 1. Calculate total tokens across all attachments
    /// 2. If under limit, return early
    /// 3. Sort non-essential attachments by priority (descending)
    /// 4. Remove attachments until under the limit
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use buildscale::services::chat::AttachmentManager;
    /// let mut manager = AttachmentManager::new();
    /// // ... add attachments ...
    /// manager.optimize_for_limit(4000);  // Keep under 4000 tokens
    /// ```
    pub fn optimize_for_limit(&mut self, max_tokens: usize) {
        let mut current_tokens: usize = self.map.values().map(|v| v.tokens).sum();

        if current_tokens <= max_tokens {
            return;
        }

        // Create a list of keys to remove, sorted by priority (descending)
        let mut candidates: Vec<(AttachmentKey, i32)> = self
            .map
            .iter()
            .filter(|(_, v)| !v.is_essential)
            .map(|(k, v)| (k.clone(), v.priority))
            .collect();

        // Sort by priority descending (highest priority value = least important)
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        for (key, _) in candidates {
            if let Some(value) = self.map.get(&key) {
                current_tokens -= value.tokens;
                self.map.shift_remove(&key);

                if current_tokens <= max_tokens {
                    break;
                }
            }
        }
    }

    /// Assembles the attachments into a final string for the LLM.
    ///
    /// Wraps workspace files in XML-like markers for clarity:
    ///
    /// ```text
    /// <file_context>
    /// File content here
    /// </file_context>
    /// ```
    pub fn render(&self) -> String {
        let mut output = String::new();

        for (key, value) in &self.map {
            match key {
                AttachmentKey::WorkspaceFile(_) => {
                    // Wrap workspace files in XML-like markers
                    output.push_str("<file_context>\n");
                    output.push_str(&value.content);
                    output.push_str("\n</file_context>\n\n");
                }
                AttachmentKey::SystemPersona => {
                    // System persona - just content
                    output.push_str(&value.content);
                    output.push_str("\n\n");
                }
                AttachmentKey::ActiveSkill(_) => {
                    // Active skill - just content
                    output.push_str(&value.content);
                    output.push_str("\n\n");
                }
                AttachmentKey::Environment => {
                    // Environment context - just content
                    output.push_str(&value.content);
                    output.push_str("\n\n");
                }
                AttachmentKey::ChatHistory => {
                    // Chat history - just content
                    output.push_str(&value.content);
                    output.push_str("\n\n");
                }
                AttachmentKey::UserRequest => {
                    // User request - just content
                    output.push_str(&value.content);
                    output.push_str("\n\n");
                }
            }
        }

        output.trim().to_string()
    }
}

/// Check if a message should be filtered from AI context.
///
/// Messages are filtered if they are only for audit/debug purposes and should
/// not be sent to the AI or shown in Context UI token counts.
///
/// Currently filters:
/// - `reasoning_complete` messages (thinking/reasoning content for audit only)
///
/// # Arguments
/// * `msg` - The chat message to check
///
/// # Returns
/// `true` if the message should be filtered out, `false` otherwise
pub fn should_filter_message(msg: &ChatMessage) -> bool {
    // Filter reasoning_complete messages - they're for audit only
    if let Some(ref message_type) = msg.metadata.message_type {
        if message_type == "reasoning_complete" {
            return true;
        }
    }
    false
}

/// Filter messages to only include those that should be in AI context.
///
/// This is the single source of truth for what messages the AI receives
/// and what the Context UI displays in token counts.
///
/// # Arguments
/// * `messages` - Slice of chat messages to filter
///
/// # Returns
/// Vector of messages that should be included in AI context
pub fn filter_messages_for_context(messages: &[ChatMessage]) -> Vec<&ChatMessage> {
    messages
        .iter()
        .filter(|msg| !should_filter_message(msg))
        .collect()
}

/// Filter messages and convert to ContextItems for unified processing.
///
/// This combines filtering with ContextItem conversion, ensuring both
/// AI context building and Context UI use the same logic.
///
/// # Arguments
/// * `messages` - Slice of chat messages to process
///
/// # Returns
/// Vector of ContextItems (only non-filtered messages)
pub fn messages_to_context_items(messages: &[ChatMessage]) -> Vec<ContextItem> {
    messages
        .iter()
        .filter(|msg| !should_filter_message(msg))
        .map(|msg| ContextItem::Message {
            role: msg.role,
            content: msg.content.clone(),
            created_at: msg.created_at,
            metadata: msg.metadata.0.clone(),
        })
        .collect()
}

/// Build sorted context items from messages and optional attachments.
///
/// This is the single source of truth for context building, used by both:
/// - AI context (rig_engine.rs) - what the AI receives
/// - Context UI (mod.rs) - what the UI displays in token counts
///
/// # Arguments
/// * `messages` - Slice of chat messages to process
/// * `attachments` - Optional attachment manager with file attachments
/// * `render_attachment` - Optional function to render attachments as strings
///
/// # Returns
/// Vector of ContextItems sorted by timestamp (oldest first)
pub fn build_sorted_context_items<F>(
    messages: &[ChatMessage],
    attachments: Option<&AttachmentManager>,
    render_attachment: Option<F>,
) -> Vec<ContextItem>
where
    F: Fn(&AttachmentKey, &AttachmentValue) -> String,
{
    let mut items = messages_to_context_items(messages);

    // Add attachments as context items (if provided)
    if let Some(att_manager) = attachments {
        for (key, value) in &att_manager.map {
            let rendered = render_attachment
                .as_ref()
                .map(|f| f(key, value))
                .unwrap_or_default();
            items.push(ContextItem::Attachment {
                key: key.clone(),
                value: value.clone(),
                rendered,
            });
        }
    }

    // Sort by timestamp (oldest first = better caching)
    items.sort_by_key(|item| item.timestamp());

    items
}

/// Render an attachment as XML for the AI context.
///
/// # Arguments
/// * `key` - The attachment key identifying the type
/// * `value` - The attachment value with content
///
/// # Returns
/// Rendered string for the AI context
pub fn render_attachment_for_ai(key: &AttachmentKey, value: &AttachmentValue) -> String {
    match key {
        AttachmentKey::WorkspaceFile(_) => {
            format!("<file_context>\n{}\n</file_context>", value.content)
        }
        AttachmentKey::SystemPersona
        | AttachmentKey::ActiveSkill(_)
        | AttachmentKey::Environment
        | AttachmentKey::ChatHistory
        | AttachmentKey::UserRequest => value.content.clone(),
    }
}

pub fn format_file_fragment(path: &str, content: &str) -> String {
    format!("File: {}\n---\n{}\n---", path, content)
}

pub fn format_history_fragment(messages: &[ChatMessage]) -> String {
    let mut history = String::from("Conversation History:\n");
    for msg in messages {
        let role = match msg.role {
            ChatMessageRole::System => "System",
            ChatMessageRole::User => "User",
            ChatMessageRole::Assistant => "Assistant",
            ChatMessageRole::Tool => "Tool",
        };
        history.push_str(&format!("{}: {}\n", role, msg.content));
    }
    history
}

/// Manages conversation history with token estimation and pruning capabilities.
///
/// # Purpose
///
/// `HistoryManager` wraps a vector of chat messages and provides:
/// - **Token Estimation**: Automatic counting using `ESTIMATED_CHARS_PER_TOKEN`
/// - **Pruning Strategies**: Future ability to truncate or summarize long conversations
/// - **Convenience Methods**: Easy access to message history
///
/// # Future Enhancements
///
/// Could be extended with:
/// - **Sliding Window**: Keep only last N messages
/// - **Smart Summarization**: Condense old messages while preserving recent ones
/// - **Token-Based Pruning**: Truncate when over token limit
/// - **Semantic Relevance**: Keep important messages regardless of age
///
/// # Example
///
/// ```text,no_run
/// use buildscale::services::chat::HistoryManager;
///
/// // Create from existing message vector
/// let manager = HistoryManager::new(vec![msg1, msg2, msg3]);
///
/// // Access as slice
/// for msg in &manager.messages {
///     println!("{}: {}", msg.role, msg.content);
/// }
///
/// // Estimate total tokens
/// let total_tokens = manager.estimate_tokens();
///
/// // Future: Prune to fit limit
/// // manager.prune_to_limit(4000);
/// ```
#[derive(Debug, Clone)]
pub struct HistoryManager {
    /// Conversation history (excluding current message)
    pub messages: Vec<ChatMessage>,
}

impl HistoryManager {
    /// Create a new HistoryManager from a message vector.
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self { messages }
    }

    /// Estimate total tokens for all messages in history.
    ///
    /// Uses `ESTIMATED_CHARS_PER_TOKEN` (4 chars per token) for approximation.
    pub fn estimate_tokens(&self) -> usize {
        self.messages
            .iter()
            .map(|msg| msg.content.len() / ESTIMATED_CHARS_PER_TOKEN)
            .sum()
    }

    /// Get the number of messages in history.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if history is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

// Implement Deref for easy access to Vec methods
impl Deref for HistoryManager {
    type Target = Vec<ChatMessage>;

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

impl DerefMut for HistoryManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.messages
    }
}

// Implement IntoIterator for convenient iteration
impl IntoIterator for HistoryManager {
    type Item = ChatMessage;
    type IntoIter = std::vec::IntoIter<ChatMessage>;

    fn into_iter(self) -> Self::IntoIter {
        self.messages.into_iter()
    }
}
