//! Plan file metadata types.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{parse_yaml_frontmatter, prepend_yaml_frontmatter};

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = r#"---
title: My Plan
status: draft
created_at: 2025-01-15T10:30:00Z
---

# Plan Content

Some content here."#;

        let (metadata, remaining) = parse_yaml_frontmatter::<PlanMetadata>(content);
        let metadata = metadata.expect("Should parse metadata");

        assert_eq!(metadata.title, "My Plan");
        assert_eq!(metadata.status, PlanStatus::Draft);
        assert!(remaining.contains("# Plan Content"));
    }

    #[test]
    fn test_parse_frontmatter_none() {
        let content = "# Just content\n\nNo frontmatter here.";
        let (metadata, remaining) = parse_yaml_frontmatter::<PlanMetadata>(content);

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

        let result = prepend_yaml_frontmatter(&metadata, content);

        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Test Plan"));
        assert!(result.contains("# My Content"));
    }
}
