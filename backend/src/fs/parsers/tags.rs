//! Tag parser for Obsidian-style #tags
//!
//! Supports:
//! - Simple tags: `#tag`
//! - Nested tags: `#parent/child`
//! - Tags with numbers: `#2024`
//!
//! Ignores:
//! - Tags inside code blocks
//! - Tags inside inline code
//! - Heading markers (# at line start)

use regex::Regex;
use std::sync::OnceLock;

/// Get the compiled tag regex (lazy initialization)
fn tag_regex() -> &'static Regex {
    static TAG_REGEX: OnceLock<Regex> = OnceLock::new();
    TAG_REGEX.get_or_init(|| {
        // Match # followed by word characters, /, -, or _
        // Will filter headings afterwards
        Regex::new(r"#[\w/\-]+").unwrap()
    })
}

/// Get the heading regex (lazy initialization)
fn heading_regex() -> &'static Regex {
    static HEADING_REGEX: OnceLock<Regex> = OnceLock::new();
    HEADING_REGEX.get_or_init(|| {
        // Match markdown headings: # at start of line followed by space
        Regex::new(r"^#{1,6}\s").unwrap()
    })
}

/// Get the code block regex (lazy initialization)
fn code_block_regex() -> &'static Regex {
    static CODE_BLOCK_REGEX: OnceLock<Regex> = OnceLock::new();
    CODE_BLOCK_REGEX.get_or_init(|| {
        Regex::new(r"```[\s\S]*?```").unwrap()
    })
}

/// Get the inline code regex (lazy initialization)
fn inline_code_regex() -> &'static Regex {
    static INLINE_CODE_REGEX: OnceLock<Regex> = OnceLock::new();
    INLINE_CODE_REGEX.get_or_init(|| {
        Regex::new(r"`[^`]+`").unwrap()
    })
}

/// Extracts tags from markdown content.
///
/// This function attempts to avoid:
/// - Heading markers (## Heading)
/// - Tags inside code blocks
/// - Tags inside inline code
///
/// # Arguments
/// * `content` - The markdown content to parse
///
/// # Returns
/// A vector of tag strings (without the # prefix), converted to lowercase
///
/// # Examples
/// ```
/// use buildscale::fs::parsers::extract_tags;
///
/// let content = "This has a #tag and #another/tag";
/// let tags = extract_tags(content);
/// assert_eq!(tags, vec!["tag", "another/tag"]);
/// ```
pub fn extract_tags(content: &str) -> Vec<String> {
    // First, remove code blocks to avoid matching tags inside them
    let content_without_code = remove_code_blocks(content);

    // Find all heading positions to filter them out
    let heading_positions: Vec<_> = heading_regex()
        .find_iter(&content_without_code)
        .map(|m| m.start())
        .collect();

    tag_regex()
        .find_iter(&content_without_code)
        .filter(|m| {
            // Filter out hashtags that are at heading positions
            let pos = m.start();
            // Check if this # is right at the start of a heading
            !heading_positions.iter().any(|&hpos| pos == hpos)
        })
        .map(|m| {
            let tag = m.as_str();
            // Remove the # prefix and convert to lowercase
            tag[1..].to_lowercase()
        })
        .filter(|tag| {
            // Filter out false positives
            // - Tags should have at least one letter
            tag.chars().any(|c| c.is_alphabetic())
        })
        .collect()
}

/// Remove code blocks from content to avoid matching tags inside them
fn remove_code_blocks(content: &str) -> String {
    let without_fenced = code_block_regex().replace_all(content, "");
    inline_code_regex()
        .replace_all(&without_fenced, "")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tag() {
        let content = "This has a #tag";
        let tags = extract_tags(content);
        assert_eq!(tags, vec!["tag"]);
    }

    #[test]
    fn test_nested_tag() {
        let content = "Nested #parent/child tag";
        let tags = extract_tags(content);
        assert_eq!(tags, vec!["parent/child"]);
    }

    #[test]
    fn test_multiple_tags() {
        let content = "Tags: #one #two #three";
        let tags = extract_tags(content);
        assert_eq!(tags, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_ignores_heading() {
        let content = "# Heading\n\nSome #tag here";
        let tags = extract_tags(content);
        assert_eq!(tags, vec!["tag"]);
    }

    #[test]
    fn test_ignores_code_blocks() {
        let content = "Some #visible\n```\n#notatag\n```\n#also-visible";
        let tags = extract_tags(content);
        assert!(tags.contains(&"visible".to_string()));
        assert!(tags.contains(&"also-visible".to_string()));
        assert!(!tags.contains(&"notatag".to_string()));
    }

    #[test]
    fn test_lowercase_conversion() {
        let content = "#UPPER and #MixedCase";
        let tags = extract_tags(content);
        assert_eq!(tags, vec!["upper", "mixedcase"]);
    }

    #[test]
    fn test_tag_with_hyphen() {
        let content = "Tag with #my-tag here";
        let tags = extract_tags(content);
        assert_eq!(tags, vec!["my-tag"]);
    }
}
