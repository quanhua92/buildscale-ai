//! Memory metadata parsing and generation for memory files.
//!
//! Memory files store persistent AI agent memories with YAML frontmatter.
//! They support two scopes:
//! - User scope: Private to a specific user (`/users/{user_id}/memories/{category}/{key}.md`)
//! - Global scope: Shared across workspace (`/memories/{category}/{key}.md`)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Memory scope determines visibility of memories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryScope {
    /// User-private memories, only visible to the creating user
    User,
    /// Global/workspace-shared memories, visible to all workspace members
    Global,
}

impl Default for MemoryScope {
    fn default() -> Self {
        Self::User
    }
}

impl std::fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryScope::User => write!(f, "user"),
            MemoryScope::Global => write!(f, "global"),
        }
    }
}

impl std::str::FromStr for MemoryScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "global" => Ok(Self::Global),
            _ => Err(format!("Invalid memory scope: {}", s)),
        }
    }
}

/// Memory metadata extracted from YAML frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Title of the memory
    pub title: String,
    /// Tags for categorization and search
    #[serde(default)]
    pub tags: Vec<String>,
    /// Category for organization
    pub category: String,
    /// When the memory was created
    pub created_at: DateTime<Utc>,
    /// When the memory was last updated
    pub updated_at: DateTime<Utc>,
    /// Scope of the memory (user or global)
    pub scope: MemoryScope,
}

/// Parse memory frontmatter from content, returns (metadata, remaining_content)
pub fn parse_memory_frontmatter(content: &str) -> (Option<MemoryMetadata>, &str) {
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

        match serde_yaml::from_str::<MemoryMetadata>(yaml_str) {
            Ok(metadata) => (Some(metadata), remaining),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse memory frontmatter");
                (None, content)
            }
        }
    } else if let Some(end_idx) = rest.find("\n---") {
        // Handle case where content ends with ---
        let yaml_str = &rest[..end_idx];
        let remaining = &rest[end_idx + 4..];

        match serde_yaml::from_str::<MemoryMetadata>(yaml_str) {
            Ok(metadata) => (Some(metadata), remaining),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse memory frontmatter");
                (None, content)
            }
        }
    } else {
        (None, content)
    }
}

/// Prepend memory frontmatter to content
pub fn prepend_memory_frontmatter(metadata: &MemoryMetadata, content: &str) -> String {
    let yaml = serde_yaml::to_string(metadata).unwrap_or_else(|_| "{}".to_string());

    // serde_yaml adds a trailing newline, so we format carefully
    let yaml = yaml.trim_end();

    if content.trim().is_empty() {
        format!("---\n{}\n---\n", yaml)
    } else {
        format!("---\n{}\n---\n{}", yaml, content)
    }
}

/// Generate memory file path based on scope, category, and key
///
/// User scope: `/users/{user_id}/memories/{category}/{key}.md`
/// Global scope: `/memories/{category}/{key}.md`
pub fn generate_memory_path(
    scope: &MemoryScope,
    category: &str,
    key: &str,
    user_id: Option<Uuid>,
) -> String {
    // Sanitize category and key for filesystem
    let sanitized_category = sanitize_path_component(category);
    let sanitized_key = sanitize_path_component(key);

    match scope {
        MemoryScope::User => {
            let uid = user_id.expect("user_id is required for user-scoped memories");
            format!("/users/{}/memories/{}/{}.md", uid, sanitized_category, sanitized_key)
        },
        MemoryScope::Global => {
            format!("/memories/{}/{}.md", sanitized_category, sanitized_key)
        }
    }
}

/// Sanitize a path component for safe filesystem use
fn sanitize_path_component(s: &str) -> String {
    // Prevent path traversal attacks
    if s == "." || s == ".." {
        return "_".to_string();
    }

    // Replace potentially problematic characters
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ' ' => '_',
            c => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// Parse memory path to extract scope, category, and key
///
/// User path: `/users/{user_id}/memories/{category}/{key}.md`
/// Global path: `/memories/{category}/{key}.md`
///
/// Returns `(scope, category, key)` if the path matches a memory file pattern.
/// Category and key are lowercased to match `generate_memory_path` behavior.
pub fn parse_memory_path(path: &str) -> Option<(MemoryScope, String, String)> {
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() >= 6 && parts[1] == "users" && parts[3] == "memories" {
        // User-scoped memory: /users/{uuid}/memories/{category}/{key}.md
        let category = parts[4].to_lowercase();
        let key = parts.get(5)?.strip_suffix(".md")?.to_lowercase();
        Some((MemoryScope::User, category, key))
    } else if parts.len() >= 4 && parts[1] == "memories" {
        // Global-scoped memory: /memories/{category}/{key}.md
        let category = parts[2].to_lowercase();
        let key = parts.get(3)?.strip_suffix(".md")?.to_lowercase();
        Some((MemoryScope::Global, category, key))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memory_frontmatter_valid() {
        let content = r#"---
title: "Meeting Notes"
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

        let (metadata, remaining) = parse_memory_frontmatter(content);
        let metadata = metadata.expect("Should parse metadata");

        assert_eq!(metadata.title, "Meeting Notes");
        assert_eq!(metadata.tags, vec!["meeting", "planning"]);
        assert_eq!(metadata.category, "work");
        assert_eq!(metadata.scope, MemoryScope::User);
        assert!(remaining.contains("# Meeting Notes"));
    }

    #[test]
    fn test_parse_memory_frontmatter_global_scope() {
        let content = r#"---
title: "Global Config"
tags: []
category: config
created_at: 2025-01-15T10:30:00Z
updated_at: 2025-01-15T10:30:00Z
scope: global
---

Configuration content."#;

        let (metadata, _) = parse_memory_frontmatter(content);
        let metadata = metadata.expect("Should parse metadata");

        assert_eq!(metadata.scope, MemoryScope::Global);
    }

    #[test]
    fn test_parse_memory_frontmatter_none() {
        let content = "# Just content\n\nNo frontmatter here.";
        let (metadata, remaining) = parse_memory_frontmatter(content);

        assert!(metadata.is_none());
        assert!(remaining.contains("Just content"));
    }

    #[test]
    fn test_prepend_memory_frontmatter() {
        let metadata = MemoryMetadata {
            title: "Test Memory".to_string(),
            tags: vec!["test".to_string()],
            category: "testing".to_string(),
            created_at: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            scope: MemoryScope::User,
        };
        let content = "# My Content\n\nBody text.";

        let result = prepend_memory_frontmatter(&metadata, content);

        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Test Memory"));
        assert!(result.contains("# My Content"));
    }

    #[test]
    fn test_generate_memory_path_user() {
        let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let path = generate_memory_path(
            &MemoryScope::User,
            "work",
            "meeting-notes",
            Some(user_id),
        );

        assert_eq!(
            path,
            "/users/00000000-0000-0000-0000-000000000001/memories/work/meeting-notes.md"
        );
    }

    #[test]
    fn test_generate_memory_path_global() {
        let path = generate_memory_path(
            &MemoryScope::Global,
            "config",
            "project-settings",
            None,
        );

        assert_eq!(path, "/memories/config/project-settings.md");
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("Meeting Notes"), "meeting_notes");
        assert_eq!(sanitize_path_component("file.txt"), "file.txt");
        assert_eq!(sanitize_path_component("path/to/file"), "path_to_file");
        assert_eq!(sanitize_path_component("special:chars?here"), "special_chars_here");
    }

    #[test]
    fn test_sanitize_path_component_traversal() {
        // Path traversal prevention
        assert_eq!(sanitize_path_component("."), "_");
        assert_eq!(sanitize_path_component(".."), "_");
    }

    #[test]
    fn test_parse_memory_path_user() {
        let path = "/users/00000000-0000-0000-0000-000000000001/memories/work/meeting.md";
        let result = parse_memory_path(path);
        assert!(result.is_some());
        let (scope, category, key) = result.unwrap();
        assert_eq!(scope, MemoryScope::User);
        assert_eq!(category, "work");
        assert_eq!(key, "meeting");
    }

    #[test]
    fn test_parse_memory_path_global() {
        let path = "/memories/config/settings.md";
        let result = parse_memory_path(path);
        assert!(result.is_some());
        let (scope, category, key) = result.unwrap();
        assert_eq!(scope, MemoryScope::Global);
        assert_eq!(category, "config");
        assert_eq!(key, "settings");
    }

    #[test]
    fn test_parse_memory_path_invalid() {
        assert!(parse_memory_path("/invalid/path").is_none());
        assert!(parse_memory_path("/memories/only-category").is_none());
    }
}
