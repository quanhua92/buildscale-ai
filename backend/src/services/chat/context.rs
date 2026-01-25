use crate::models::chat::{ChatMessage, ChatMessageRole};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::ops::{Deref, DerefMut};

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
                    output.push_str("<file_context>\n");
                    output.push_str(&value.content);
                    output.push_str("\n</file_context>\n\n");
                }
                _ => {
                    output.push_str(&value.content);
                    output.push_str("\n\n");
                }
            }
        }

        output.trim().to_string()
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
