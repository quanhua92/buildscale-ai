//! YAML Frontmatter Sync for Chat Metadata
//!
//! This module handles bidirectional synchronization between:
//! - Database storage (source of truth via file content)
//! - YAML frontmatter in .chat files (for display/debugging)
//!
//! Chat metadata (mode, plan_file, model) is serialized to YAML frontmatter
//! when saving and parsed when loading to provide human-readable
//! configuration in .chat files.
//!
//! # Simplified Schema
//!
//! In the simplified schema, the AgentConfig is stored in the file content
//! as YAML frontmatter, not in a separate version table. This module provides
//! helpers to read/write AgentConfig from/to file content.

use crate::models::chat::AgentConfig;
use crate::error::Result;
use crate::services::storage::FileStorageService;
use crate::queries;
use crate::utils::{parse_yaml_frontmatter, prepend_yaml_frontmatter};
use crate::DbConn;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Default AgentConfig for new chats
fn default_agent_config() -> AgentConfig {
    AgentConfig {
        agent_id: None,
        model: crate::models::chat::DEFAULT_CHAT_MODEL.to_string(),
        temperature: 0.7,
        persona_override: None,
        previous_response_id: None,
        mode: "plan".to_string(),
        plan_file: None,
    }
}

/// Get AgentConfig from a chat file's content
///
/// Reads the file content and parses the YAML frontmatter to extract AgentConfig.
/// Returns default config if file has no frontmatter or parsing fails.
pub async fn get_agent_config_from_file(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    chat_file_id: Uuid,
) -> Result<AgentConfig> {
    // 1. Get the file
    let file = queries::files::get_file_by_id(conn, chat_file_id).await?;

    // 2. Read file content
    let content_bytes = match storage.read_file(workspace_id, &file.path).await {
        Ok(bytes) => bytes,
        Err(_) => {
            // File doesn't exist yet, return default config
            return Ok(default_agent_config());
        }
    };
    let content = String::from_utf8(content_bytes).unwrap_or_default();

    // 3. Parse YAML frontmatter using shared utility
    let (metadata, _) = parse_yaml_frontmatter::<ChatFrontmatter>(&content);

    // 4. Convert to AgentConfig (use default if no frontmatter)
    let frontmatter = metadata.unwrap_or_else(ChatFrontmatter::default);
    Ok(frontmatter.to_agent_config())
}

/// Update AgentConfig in a chat file's content
///
/// Reads current file content, updates the YAML frontmatter with new config,
/// and writes it back. Preserves the body content of the file.
pub async fn update_agent_config_in_file(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    chat_file_id: Uuid,
    new_config: &AgentConfig,
) -> Result<()> {
    // 1. Get the file
    let file = queries::files::get_file_by_id(conn, chat_file_id).await?;

    // 2. Read current file content
    let current_content = match storage.read_file(workspace_id, &file.path).await {
        Ok(bytes) => String::from_utf8(bytes).unwrap_or_default(),
        Err(_) => String::new(),
    };

    // 3. Parse existing content to get body using shared utility
    // The tuple (frontmatter, body) returns the remaining content after frontmatter
    let (_, body_content) = parse_yaml_frontmatter::<ChatFrontmatter>(&current_content);
    let body_content = body_content.to_string();

    // 4. Create new frontmatter with updated config using shared utility
    let frontmatter = ChatFrontmatter::from_agent_config(new_config);
    let new_content = prepend_yaml_frontmatter(&frontmatter, &body_content);

    // 5. Update file content (this creates a new version in the simplified schema)
    crate::services::files::update_file_content(
        conn,
        storage,
        chat_file_id,
        serde_json::json!(new_content),
    ).await?;

    Ok(())
}

/// YAML frontmatter structure for chat metadata
///
/// This structure represents the YAML frontmatter that appears
/// at the top of .chat files, providing human-readable configuration.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ChatFrontmatter {
    /// Chat mode: "plan" or "build"
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Path to associated plan file (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_file: Option<String>,
    /// AI model identifier (e.g., "openrouter:openai/gpt-oss-120b")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Additional metadata (for future extensibility)
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for ChatFrontmatter {
    fn default() -> Self {
        ChatFrontmatter {
            mode: "plan".to_string(),
            plan_file: None,
            model: None,
            extra: std::collections::HashMap::new(),
        }
    }
}

fn default_mode() -> String {
    "plan".to_string()
}

impl ChatFrontmatter {
    /// Create frontmatter from AgentConfig
    pub fn from_agent_config(config: &AgentConfig) -> Self {
        ChatFrontmatter {
            mode: config.mode.clone(),
            plan_file: config.plan_file.clone(),
            model: if config.model.is_empty() { None } else { Some(config.model.clone()) },
            extra: std::collections::HashMap::new(),
        }
    }

    /// Convert to AgentConfig
    ///
    /// Note: This extracts mode, plan_file, and model from frontmatter.
    /// Other AgentConfig fields (temperature, etc.) are set to defaults.
    pub fn to_agent_config(&self) -> AgentConfig {
        AgentConfig {
            agent_id: None,
            model: self.model.clone().unwrap_or_default(),
            temperature: 0.7,
            persona_override: None,
            previous_response_id: None,
            mode: self.mode.clone(),
            plan_file: self.plan_file.clone(),
        }
    }

    /// Merge frontmatter into existing AgentConfig
    ///
    /// Updates mode, plan_file, and model while preserving other fields.
    pub fn merge_into_agent_config(&self, mut config: AgentConfig) -> AgentConfig {
        config.mode = self.mode.clone();
        config.plan_file = self.plan_file.clone();
        if let Some(model) = &self.model {
            config.model = model.clone();
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontmatter_roundtrip() {
        let frontmatter = ChatFrontmatter {
            mode: "plan".to_string(),
            plan_file: Some("/plans/my-plan.plan".to_string()),
            model: Some("openrouter:deepseek/deepseek-r1:free".to_string()),
            extra: std::collections::HashMap::new(),
        };

        let yaml = serde_yaml::to_string(&frontmatter).unwrap();
        let parsed: ChatFrontmatter = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(frontmatter, parsed);
    }

    #[test]
    fn test_parse_with_frontmatter() {
        let content = r#"---
mode: build
plan_file: /plans/example.plan
---
Some chat content here"#;

        let (metadata, remaining) = parse_yaml_frontmatter::<ChatFrontmatter>(content);
        let metadata = metadata.expect("Should parse frontmatter");
        assert_eq!(metadata.mode, "build");
        assert_eq!(metadata.plan_file, Some("/plans/example.plan".to_string()));
        assert!(remaining.contains("Some chat content here"));
    }

    #[test]
    fn test_parse_without_frontmatter() {
        let content = "Just regular content without frontmatter";

        let (metadata, remaining) = parse_yaml_frontmatter::<ChatFrontmatter>(content);
        assert!(metadata.is_none());
        assert_eq!(remaining, content);
    }

    #[test]
    fn test_default_frontmatter() {
        let default = ChatFrontmatter::default();
        assert_eq!(default.mode, "plan");
        assert_eq!(default.plan_file, None);
        assert_eq!(default.model, None);
        assert!(default.extra.is_empty());
    }

    #[test]
    fn test_serialize_and_parse() {
        let frontmatter = ChatFrontmatter {
            mode: "build".to_string(),
            plan_file: Some("/plans/test.plan".to_string()),
            model: Some("openrouter:deepseek/deepseek-r1:free".to_string()),
            extra: std::collections::HashMap::new(),
        };

        let serialized = prepend_yaml_frontmatter(&frontmatter, "Chat content");
        let (parsed, remaining) = parse_yaml_frontmatter::<ChatFrontmatter>(&serialized);

        let parsed = parsed.expect("Should parse roundtripped content");
        assert_eq!(parsed.mode, frontmatter.mode);
        assert_eq!(parsed.plan_file, frontmatter.plan_file);
        assert!(remaining.contains("Chat content"));
    }
}
