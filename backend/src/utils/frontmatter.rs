//! YAML frontmatter parsing and generation for plan files.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Plan status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Draft,
    Approved,
    Implemented,
    Archived,
}

impl Default for PlanStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl FromStr for PlanStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "approved" => Ok(Self::Approved),
            "implemented" => Ok(Self::Implemented),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("Invalid plan status: {}", s)),
        }
    }
}

/// Plan metadata extracted from YAML frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    pub title: String,
    #[serde(default)]
    pub status: PlanStatus,
    pub created_at: DateTime<Utc>,
}

/// Parse frontmatter from content, returns (metadata, remaining_content)
pub fn parse_frontmatter(content: &str) -> (Option<PlanMetadata>, &str) {
    let content = content.trim_start();

    // Check for YAML frontmatter delimiter
    if !content.starts_with("---\n") {
        return (None, content);
    }

    // Find closing delimiter
    let rest = &content[4..]; // Skip opening "---\n"
    if let Some(end_idx) = rest.find("\n---\n") {
        let yaml_str = &rest[..end_idx];
        let remaining = &rest[end_idx + 5..]; // Skip "\n---\n"

        match serde_yaml::from_str::<PlanMetadata>(yaml_str) {
            Ok(metadata) => (Some(metadata), remaining),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse frontmatter");
                (None, content)
            }
        }
    } else if let Some(end_idx) = rest.find("\n---") {
        // Handle case where content ends with ---
        let yaml_str = &rest[..end_idx];
        let remaining = &rest[end_idx + 4..];

        match serde_yaml::from_str::<PlanMetadata>(yaml_str) {
            Ok(metadata) => (Some(metadata), remaining),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse frontmatter");
                (None, content)
            }
        }
    } else {
        (None, content)
    }
}

/// Prepend frontmatter to content
pub fn prepend_frontmatter(metadata: &PlanMetadata, content: &str) -> String {
    let yaml = serde_yaml::to_string(metadata).unwrap_or_else(|_| "{}".to_string());

    // serde_yaml adds a trailing newline, so we format carefully
    let yaml = yaml.trim_end();

    if content.trim().is_empty() {
        format!("---\n{}\n---\n", yaml)
    } else {
        format!("---\n{}\n---\n{}", yaml, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = r#"---
title: My Plan
status: draft
created_at: 2025-01-15T10:30:00Z
---

# Plan Content

Some content here."#;

        let (metadata, remaining) = parse_frontmatter(content);
        let metadata = metadata.expect("Should parse metadata");

        assert_eq!(metadata.title, "My Plan");
        assert_eq!(metadata.status, PlanStatus::Draft);
        assert!(remaining.contains("# Plan Content"));
    }

    #[test]
    fn test_parse_frontmatter_none() {
        let content = "# Just content\n\nNo frontmatter here.";
        let (metadata, remaining) = parse_frontmatter(content);

        assert!(metadata.is_none());
        assert!(remaining.contains("Just content"));
    }

    #[test]
    fn test_prepend_frontmatter() {
        let metadata = PlanMetadata {
            title: "Test Plan".to_string(),
            status: PlanStatus::Draft,
            created_at: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let content = "# My Content\n\nBody text.";

        let result = prepend_frontmatter(&metadata, content);

        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Test Plan"));
        assert!(result.contains("# My Content"));
    }
}
