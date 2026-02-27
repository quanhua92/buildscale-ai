//! Shared YAML frontmatter parsing and generation utilities.
//!
//! This module provides generic functions for parsing and generating YAML
//! frontmatter in files. Frontmatter is delimited by `---` markers at the
//! beginning of a file:
//!
//! ```yaml
//! ---
//! title: My Document
//! created_at: 2025-01-15T10:30:00Z
//! ---
//!
//! Content here...
//! ```

use serde::{de::DeserializeOwned, Serialize};

/// Parse YAML frontmatter from content.
///
/// Returns `(Some(metadata), remaining_content)` if valid frontmatter is found.
/// Returns `(None, original_content)` if no frontmatter or if parsing fails.
///
/// # Arguments
///
/// * `content` - The file content to parse
///
/// # Returns
///
/// A tuple of (optional metadata, remaining content after frontmatter)
///
/// # Example
///
/// ```
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Deserialize)]
/// struct MyMetadata {
///     title: String,
/// }
///
/// let content = r#"---
/// title: My Document
/// ---
///
/// Body content"#;
///
/// let (metadata, body) = buildscale::utils::yaml_frontmatter::parse_yaml_frontmatter::<MyMetadata>(content);
/// assert!(metadata.is_some());
/// assert!(body.contains("Body content"));
/// ```
pub fn parse_yaml_frontmatter<T: DeserializeOwned>(content: &str) -> (Option<T>, &str) {
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

        match serde_yaml::from_str::<T>(yaml_str) {
            Ok(metadata) => (Some(metadata), remaining),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse frontmatter");
                (None, content)
            }
        }
    } else if let Some(end_idx) = rest.find("\n---") {
        // Handle case where content ends with --- (no trailing newline after delimiter)
        let yaml_str = &rest[..end_idx];
        let remaining = &rest[end_idx + 4..]; // Skip "\n---"

        match serde_yaml::from_str::<T>(yaml_str) {
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

/// Prepend YAML frontmatter to content.
///
/// Serializes the metadata to YAML and wraps it with `---` delimiters.
///
/// # Arguments
///
/// * `metadata` - The metadata to serialize
/// * `content` - The body content to prepend frontmatter to
///
/// # Returns
///
/// A string with YAML frontmatter followed by the body content
///
/// # Example
///
/// ```
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize)]
/// struct MyMetadata {
///     title: String,
/// }
///
/// let metadata = MyMetadata { title: "My Doc".to_string() };
/// let body = "Content here";
///
/// let result = buildscale::utils::yaml_frontmatter::prepend_yaml_frontmatter(&metadata, body);
/// assert!(result.starts_with("---\n"));
/// assert!(result.contains("title: My Doc"));
/// assert!(result.contains("Content here"));
/// ```
pub fn prepend_yaml_frontmatter<T: Serialize>(metadata: &T, content: &str) -> String {
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
    use crate::utils::{PlanMetadata, PlanStatus, MemoryMetadata, MemoryScope};
    use chrono::{DateTime, Utc};

    #[test]
    fn test_parse_yaml_frontmatter_plan() {
        let content = r#"---
title: Test Plan
status: draft
created_at: 2025-01-15T10:30:00Z
---

# Body Content

Some text here."#;

        let (metadata, remaining) = parse_yaml_frontmatter::<PlanMetadata>(content);
        let metadata = metadata.expect("Should parse metadata");

        assert_eq!(metadata.title, "Test Plan");
        assert_eq!(metadata.status, PlanStatus::Draft);
        assert!(remaining.contains("# Body Content"));
    }

    #[test]
    fn test_parse_yaml_frontmatter_none() {
        let content = "# Just content\n\nNo frontmatter here.";
        let (metadata, remaining) = parse_yaml_frontmatter::<PlanMetadata>(content);

        assert!(metadata.is_none());
        assert!(remaining.contains("Just content"));
    }

    #[test]
    fn test_parse_yaml_frontmatter_invalid_yaml() {
        let content = "---\ninvalid: [yaml: syntax\n---\n\nBody";
        let (metadata, remaining) = parse_yaml_frontmatter::<PlanMetadata>(content);

        // Should return None for invalid YAML
        assert!(metadata.is_none());
        // Should return original content on parse failure
        assert!(remaining.contains("---"));
    }

    #[test]
    fn test_parse_yaml_frontmatter_no_closing_delimiter() {
        let content = "---\ntitle: Test\n\nBody without closing delimiter";
        let (metadata, remaining) = parse_yaml_frontmatter::<PlanMetadata>(content);

        assert!(metadata.is_none());
        assert!(remaining.contains("---"));
    }

    #[test]
    fn test_parse_yaml_frontmatter_ends_with_delimiter() {
        let content = "---\ntitle: Test\nstatus: draft\ncreated_at: 2025-01-15T10:30:00Z\n---";
        let (metadata, remaining) = parse_yaml_frontmatter::<PlanMetadata>(content);

        let metadata = metadata.expect("Should parse metadata");
        assert_eq!(metadata.title, "Test");
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_prepend_yaml_frontmatter_with_content() {
        let metadata = PlanMetadata {
            title: "My Title".to_string(),
            status: PlanStatus::Draft,
            created_at: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let content = "# Heading\n\nBody text.";

        let result = prepend_yaml_frontmatter(&metadata, content);

        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: My Title"));
        assert!(result.contains("# Heading"));
    }

    #[test]
    fn test_prepend_yaml_frontmatter_empty_content() {
        let metadata = PlanMetadata {
            title: "My Title".to_string(),
            status: PlanStatus::Draft,
            created_at: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let content = "";

        let result = prepend_yaml_frontmatter(&metadata, content);

        assert!(result.starts_with("---\n"));
        assert!(result.ends_with("---\n"));
        assert!(result.contains("title: My Title"));
    }

    #[test]
    fn test_roundtrip_plan() {
        let original = PlanMetadata {
            title: "Roundtrip Test".to_string(),
            status: PlanStatus::Approved,
            created_at: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let body = "\n# Content\n\nSome body text.";

        let serialized = prepend_yaml_frontmatter(&original, body);
        let (parsed, remaining) = parse_yaml_frontmatter::<PlanMetadata>(&serialized);

        let parsed = parsed.expect("Should parse roundtripped content");
        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.status, original.status);
        assert_eq!(remaining, body);
    }

    #[test]
    fn test_parse_yaml_frontmatter_memory() {
        let content = r#"---
title: Meeting Notes
tags:
  - meeting
  - planning
category: work
created_at: 2025-01-15T10:30:00Z
updated_at: 2025-01-15T10:30:00Z
scope: user
---

# Meeting Notes

Some content here."#;

        let (metadata, remaining) = parse_yaml_frontmatter::<MemoryMetadata>(content);
        let metadata = metadata.expect("Should parse metadata");

        assert_eq!(metadata.title, "Meeting Notes");
        assert_eq!(metadata.tags, vec!["meeting", "planning"]);
        assert_eq!(metadata.category, "work");
        assert_eq!(metadata.scope, MemoryScope::User);
        assert!(remaining.contains("# Meeting Notes"));
    }

    #[test]
    fn test_roundtrip_memory() {
        let original = MemoryMetadata {
            title: "Test Memory".to_string(),
            tags: vec!["test".to_string()],
            category: "testing".to_string(),
            created_at: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            scope: MemoryScope::Global,
        };
        let body = "Memory content";

        let serialized = prepend_yaml_frontmatter(&original, body);
        let (parsed, remaining) = parse_yaml_frontmatter::<MemoryMetadata>(&serialized);

        let parsed = parsed.expect("Should parse roundtripped content");
        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.tags, original.tags);
        assert_eq!(parsed.category, original.category);
        assert_eq!(parsed.scope, original.scope);
        assert_eq!(remaining, body);
    }
}
