use crate::models::chat::{ChatMessage, ChatMessageRole};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "lowercase")]
pub enum ContextKey {
    SystemPersona,
    ActiveSkill(Uuid),
    WorkspaceFile(Uuid),
    Environment,
    ChatHistory,
    UserRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextValue {
    pub content: String,
    pub priority: i32,
    pub tokens: usize,
    pub is_essential: bool,
}

pub type ContextMap = IndexMap<ContextKey, ContextValue>;

// --- Context Engineering Constants ---

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

pub struct ContextManager {
    pub map: ContextMap,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            map: IndexMap::new(),
        }
    }

    pub fn add_fragment(&mut self, key: ContextKey, value: ContextValue) {
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

    fn get_key_position(key: &ContextKey) -> i32 {
        match key {
            ContextKey::SystemPersona => POS_SYSTEM_PERSONA,
            ContextKey::ActiveSkill(_) => POS_SKILLS,
            ContextKey::WorkspaceFile(_) => POS_WORKSPACE_FILES,
            ContextKey::Environment => POS_ENVIRONMENT,
            ContextKey::ChatHistory => POS_CHAT_HISTORY,
            ContextKey::UserRequest => POS_USER_REQUEST,
        }
    }

    /// Prunes the context to fit within a token limit.
    /// Removes fragments with the highest priority values (lowest importance)
    /// that are not marked as essential.
    pub fn optimize_for_limit(&mut self, max_tokens: usize) {
        let mut current_tokens: usize = self.map.values().map(|v| v.tokens).sum();

        if current_tokens <= max_tokens {
            return;
        }

        // Create a list of keys to remove, sorted by priority (descending)
        let mut candidates: Vec<(ContextKey, i32)> = self
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

    /// Assembles the fragments into a final string for the LLM.
    /// Wraps workspace files in XML-like markers for clarity.
    pub fn render(&self) -> String {
        let mut output = String::new();

        for (key, value) in &self.map {
            match key {
                ContextKey::WorkspaceFile(_) => {
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
