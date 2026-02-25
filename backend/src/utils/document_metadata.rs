//! Document metadata types for Obsidian-style frontmatter.
//!
//! Documents support YAML frontmatter with properties like title, tags, and timestamps.
//! This is compatible with Obsidian's property system and supports arbitrary custom fields.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Document metadata extracted from YAML frontmatter (Obsidian-style)
///
/// Supports both standard properties (title, tags, aliases, created, modified)
/// and arbitrary custom properties via the `extra` field.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DocumentMetadata {
    /// Document title
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// Tags for categorization and search
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Alternative names for this document
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Creation timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,
    /// Last modification timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
    /// Arbitrary custom properties (Obsidian-style)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

impl DocumentMetadata {
    /// Create metadata with title and timestamps
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            title: title.into(),
            created: Some(now),
            modified: Some(now),
            ..Default::default()
        }
    }

    /// Create metadata from a filename (derives title)
    pub fn from_filename(filename: &str) -> Self {
        // Remove extension for title
        let title = filename
            .rsplit_once('.')
            .map(|(name, _)| name)
            .unwrap_or(filename)
            .replace('-', " ")
            .replace('_', " ");

        let now = Utc::now();
        Self {
            title,
            created: Some(now),
            modified: Some(now),
            ..Default::default()
        }
    }

    /// Update modification timestamp
    pub fn touch(&mut self) {
        self.modified = Some(Utc::now());
    }

    /// Check if metadata has any content worth serializing
    pub fn is_empty(&self) -> bool {
        self.title.is_empty()
            && self.tags.is_empty()
            && self.aliases.is_empty()
            && self.created.is_none()
            && self.modified.is_none()
            && self.extra.is_empty()
    }

    /// Get a property by key (works with standard and custom fields)
    pub fn get(&self, key: &str) -> Option<serde_yaml::Value> {
        match key.to_lowercase().as_str() {
            "title" => {
                if self.title.is_empty() { None } else { Some(serde_yaml::Value::String(self.title.clone())) }
            }
            "tags" => {
                if self.tags.is_empty() { None } else { serde_yaml::to_value(&self.tags).ok() }
            }
            "aliases" => {
                if self.aliases.is_empty() { None } else { serde_yaml::to_value(&self.aliases).ok() }
            }
            "created" => {
                self.created.map(|dt| serde_yaml::Value::String(dt.to_rfc3339()))
            }
            "modified" => {
                self.modified.map(|dt| serde_yaml::Value::String(dt.to_rfc3339()))
            }
            _ => self.extra.get(key).cloned(),
        }
    }

    /// Set a property by key (works with standard and custom fields)
    pub fn set(&mut self, key: &str, value: serde_yaml::Value) {
        match key.to_lowercase().as_str() {
            "title" => {
                self.title = value.as_str().unwrap_or("").to_string();
            }
            "tags" => {
                self.tags = serde_yaml::from_value(value).unwrap_or_default();
            }
            "aliases" => {
                self.aliases = serde_yaml::from_value(value).unwrap_or_default();
            }
            "created" => {
                if let Some(s) = value.as_str() {
                    self.created = DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc));
                }
            }
            "modified" => {
                if let Some(s) = value.as_str() {
                    self.modified = DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc));
                }
            }
            _ => {
                self.extra.insert(key.to_string(), value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{parse_yaml_frontmatter, prepend_yaml_frontmatter};

    #[test]
    fn test_document_metadata_new() {
        let meta = DocumentMetadata::new("My Document");
        assert_eq!(meta.title, "My Document");
        assert!(meta.created.is_some());
        assert!(meta.modified.is_some());
    }

    #[test]
    fn test_document_metadata_from_filename() {
        let meta = DocumentMetadata::from_filename("my-notes.md");
        assert_eq!(meta.title, "my notes");
    }

    #[test]
    fn test_document_metadata_empty() {
        let meta = DocumentMetadata::default();
        assert!(meta.is_empty());
    }

    #[test]
    fn test_document_metadata_not_empty_with_title() {
        let meta = DocumentMetadata {
            title: "Test".to_string(),
            ..Default::default()
        };
        assert!(!meta.is_empty());
    }

    #[test]
    fn test_parse_document_frontmatter() {
        let content = r#"---
title: My Notes
tags:
  - work
  - ideas
aliases:
  - notes
created: 2025-01-15T10:30:00Z
modified: 2025-01-16T14:20:00Z
---

# My Notes

Some content here."#;

        let (metadata, body) = parse_yaml_frontmatter::<DocumentMetadata>(content);
        let metadata = metadata.expect("Should parse metadata");

        assert_eq!(metadata.title, "My Notes");
        assert_eq!(metadata.tags, vec!["work", "ideas"]);
        assert_eq!(metadata.aliases, vec!["notes"]);
        assert!(body.contains("# My Notes"));
    }

    #[test]
    fn test_parse_document_with_custom_fields() {
        let content = r#"---
title: Project Notes
priority: high
assignee: alice
estimate: 5
---

Content here."#;

        let (metadata, _) = parse_yaml_frontmatter::<DocumentMetadata>(content);
        let metadata = metadata.expect("Should parse metadata");

        assert_eq!(metadata.title, "Project Notes");
        assert_eq!(metadata.get("priority").unwrap().as_str().unwrap(), "high");
        assert_eq!(metadata.get("assignee").unwrap().as_str().unwrap(), "alice");
        assert_eq!(metadata.get("estimate").unwrap().as_i64().unwrap(), 5);
    }

    #[test]
    fn test_parse_document_no_frontmatter() {
        let content = "# Just a document\n\nNo frontmatter here.";
        let (metadata, body) = parse_yaml_frontmatter::<DocumentMetadata>(content);

        assert!(metadata.is_none());
        assert!(body.contains("Just a document"));
    }

    #[test]
    fn test_prepend_document_frontmatter() {
        let meta = DocumentMetadata {
            title: "Test Doc".to_string(),
            tags: vec!["test".to_string()],
            aliases: vec![],
            created: Some(DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc)),
            modified: None,
            extra: HashMap::new(),
        };

        let result = prepend_yaml_frontmatter(&meta, "Content here");

        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Test Doc"));
        assert!(result.contains("tags:"));
        assert!(result.contains("- test"));
        assert!(result.contains("Content here"));
    }

    #[test]
    fn test_roundtrip() {
        let original = DocumentMetadata {
            title: "Roundtrip Test".to_string(),
            tags: vec!["one".to_string(), "two".to_string()],
            aliases: vec!["rt".to_string()],
            created: Some(DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc)),
            modified: Some(DateTime::parse_from_rfc3339("2025-01-16T14:20:00Z")
                .unwrap()
                .with_timezone(&Utc)),
            extra: HashMap::new(),
        };

        let serialized = prepend_yaml_frontmatter(&original, "\n# Content\n\nBody text.");
        let (parsed, body) = parse_yaml_frontmatter::<DocumentMetadata>(&serialized);

        let parsed = parsed.expect("Should parse roundtripped content");
        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.tags, original.tags);
        assert_eq!(parsed.aliases, original.aliases);
        assert!(body.contains("# Content"));
    }

    #[test]
    fn test_touch_updates_modified() {
        let mut meta = DocumentMetadata::new("Test");
        let first_modified = meta.modified;
        std::thread::sleep(std::time::Duration::from_millis(10));
        meta.touch();
        assert!(meta.modified > first_modified);
    }

    #[test]
    fn test_get_standard_field_title() {
        let meta = DocumentMetadata::new("My Title");
        let value = meta.get("title").unwrap();
        assert_eq!(value.as_str().unwrap(), "My Title");
    }

    #[test]
    fn test_get_standard_field_case_insensitive() {
        let meta = DocumentMetadata::new("My Title");
        assert_eq!(meta.get("TITLE").unwrap().as_str().unwrap(), "My Title");
        assert_eq!(meta.get("Title").unwrap().as_str().unwrap(), "My Title");
    }

    #[test]
    fn test_get_custom_field() {
        let mut meta = DocumentMetadata::new("Test");
        meta.set("priority", serde_yaml::Value::String("high".to_string()));
        assert_eq!(meta.get("priority").unwrap().as_str().unwrap(), "high");
    }

    #[test]
    fn test_set_standard_field() {
        let mut meta = DocumentMetadata::default();
        meta.set("title", serde_yaml::Value::String("New Title".to_string()));
        assert_eq!(meta.title, "New Title");

        meta.set("tags", serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("a".to_string()),
            serde_yaml::Value::String("b".to_string()),
        ]));
        assert_eq!(meta.tags, vec!["a", "b"]);
    }

    #[test]
    fn test_set_custom_field() {
        let mut meta = DocumentMetadata::new("Test");
        meta.set("custom_field", serde_yaml::Value::Number(42.into()));
        assert_eq!(meta.get("custom_field").unwrap().as_i64().unwrap(), 42);
    }

    #[test]
    fn test_get_missing_field() {
        let meta = DocumentMetadata::default();
        assert!(meta.get("nonexistent").is_none());
        assert!(meta.get("title").is_none()); // Empty title returns None
    }
}
