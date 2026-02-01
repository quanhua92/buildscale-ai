//! YAML Frontmatter Sync for Chat Metadata
//!
//! This module handles bidirectional synchronization between:
//! - Database storage (source of truth)
//! - YAML frontmatter in .chat files (for display/debugging)
//!
//! Chat metadata (mode, plan_file) is serialized to YAML frontmatter
//! when saving and parsed when loading to provide human-readable
//! configuration in .chat files.

use crate::models::chat::AgentConfig;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::str;

/// YAML frontmatter structure for chat metadata
///
/// This structure represents the YAML frontmatter that appears
/// at the top of .chat files, providing human-readable configuration.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ChatFrontmatter {
    /// Chat mode: "plan" or "build"
    pub mode: String,
    /// Path to associated plan file (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_file: Option<String>,
    /// Additional metadata (for future extensibility)
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl ChatFrontmatter {
    /// Create frontmatter from AgentConfig
    pub fn from_agent_config(config: &AgentConfig) -> Self {
        ChatFrontmatter {
            mode: config.mode.clone(),
            plan_file: config.plan_file.clone(),
            extra: std::collections::HashMap::new(),
        }
    }

    /// Convert to AgentConfig
    ///
    /// Note: This only extracts mode and plan_file. Other AgentConfig
    /// fields (model, temperature, etc.) should be preserved from the
    /// existing config or set to defaults.
    pub fn to_agent_config(&self) -> AgentConfig {
        AgentConfig {
            agent_id: None,
            model: String::new(), // Caller should set this
            temperature: 0.7,
            persona_override: None,
            previous_response_id: None,
            mode: self.mode.clone(),
            plan_file: self.plan_file.clone(),
        }
    }

    /// Merge frontmatter into existing AgentConfig
    ///
    /// Updates mode and plan_file while preserving other fields.
    pub fn merge_into_agent_config(&self, mut config: AgentConfig) -> AgentConfig {
        config.mode = self.mode.clone();
        config.plan_file = self.plan_file.clone();
        config
    }
}

/// YAML frontmatter wrapper with delimiter markers
#[derive(Debug, Clone)]
pub struct YamlFrontmatter {
    pub frontmatter: ChatFrontmatter,
    pub content: String,
}

impl YamlFrontmatter {
    const DELIMITER_START: &'static str = "---";
    const DELIMITER_END: &'static str = "---";

    /// Create new YAML frontmatter with content
    pub fn new(frontmatter: ChatFrontmatter, content: String) -> Self {
        Self {
            frontmatter,
            content,
        }
    }

    /// Parse YAML frontmatter from file content
    ///
    /// Expected format:
    /// ```yaml
    /// ---
    /// mode: plan
    /// plan_file: /plans/my-plan.plan
    /// ---
    /// [rest of file content]
    /// ```
    pub fn parse(content: &str) -> Result<Self> {
        let trimmed = content.trim_start();

        if !trimmed.starts_with(Self::DELIMITER_START) {
            // No frontmatter, treat entire content as body
            return Ok(Self {
                frontmatter: ChatFrontmatter {
                    mode: "plan".to_string(), // Default mode
                    plan_file: None,
                    extra: std::collections::HashMap::new(),
                },
                content: content.to_string(),
            });
        }

        // Find the end delimiter
        let after_start = trimmed[Self::DELIMITER_START.len()..].trim_start();
        let end_pos = after_start
            .find(Self::DELIMITER_END)
            .ok_or_else(|| {
                Error::Validation(crate::error::ValidationErrors::Single {
                    field: "content".to_string(),
                    message: "Unclosed YAML frontmatter delimiter (missing closing '---')".to_string(),
                })
            })?;

        let yaml_str = &after_start[..end_pos];
        let body_content = after_start[end_pos + Self::DELIMITER_END.len()..].trim_start();

        // Parse YAML
        let frontmatter: ChatFrontmatter = serde_yaml::from_str(yaml_str).map_err(|e| {
            Error::Validation(crate::error::ValidationErrors::Single {
                field: "yaml_frontmatter".to_string(),
                message: format!("Failed to parse YAML frontmatter: {}", e),
            })
        })?;

        Ok(Self {
            frontmatter,
            content: body_content.to_string(),
        })
    }

    /// Serialize to YAML frontmatter format
    pub fn serialize(&self) -> Result<String> {
        let yaml_str = serde_yaml::to_string(&self.frontmatter).map_err(|e| {
            Error::Internal(format!("Failed to serialize YAML frontmatter: {}", e))
        })?;

        Ok(format!(
            "{}\n{}\n{}\n{}",
            Self::DELIMITER_START,
            yaml_str.trim(),
            Self::DELIMITER_END,
            if self.content.is_empty() {
                String::new()
            } else {
                format!("\n{}", self.content)
            }
        ))
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

        let parsed = YamlFrontmatter::parse(content).unwrap();
        assert_eq!(parsed.frontmatter.mode, "build");
        assert_eq!(parsed.frontmatter.plan_file, Some("/plans/example.plan".to_string()));
        assert_eq!(parsed.content.trim(), "Some chat content here");
    }

    #[test]
    fn test_parse_without_frontmatter() {
        let content = "Just regular content without frontmatter";

        let parsed = YamlFrontmatter::parse(content).unwrap();
        assert_eq!(parsed.frontmatter.mode, "plan"); // Default
        assert_eq!(parsed.frontmatter.plan_file, None);
        assert_eq!(parsed.content, content);
    }

    #[test]
    fn test_serialize_and_parse() {
        let frontmatter = ChatFrontmatter {
            mode: "build".to_string(),
            plan_file: Some("/plans/test.plan".to_string()),
            extra: std::collections::HashMap::new(),
        };

        let yaml_frontmatter = YamlFrontmatter::new(frontmatter, "Chat content".to_string());
        let serialized = yaml_frontmatter.serialize().unwrap();
        let reparsed = YamlFrontmatter::parse(&serialized).unwrap();

        assert_eq!(yaml_frontmatter.frontmatter, reparsed.frontmatter);
        assert_eq!(reparsed.content.trim(), "Chat content");
    }
}
