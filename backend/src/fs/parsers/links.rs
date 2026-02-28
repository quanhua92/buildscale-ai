//! Wikilink parser for Obsidian-style [[links]]
//!
//! Supports:
//! - Simple links: `[[note]]`
//! - Links with display text: `[[note|display text]]`
//! - Links with headers: `[[note#header]]`
//! - Links with headers and display: `[[note#header|display]]`

use regex::Regex;
use std::sync::OnceLock;

/// Get the compiled wikilink regex (lazy initialization)
fn wikilink_regex() -> &'static Regex {
    static WIKILINK_REGEX: OnceLock<Regex> = OnceLock::new();
    WIKILINK_REGEX.get_or_init(|| {
        // Matches [[link]] or [[link|display]] or [[link#header|display]]
        Regex::new(r"\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|[^\]]+)?\]\]").unwrap()
    })
}

/// Get the full wikilink regex with display capture (lazy initialization)
fn wikilink_full_regex() -> &'static Regex {
    static WIKILINK_FULL_REGEX: OnceLock<Regex> = OnceLock::new();
    WIKILINK_FULL_REGEX.get_or_init(|| {
        Regex::new(r"\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|([^\]]+))?\]\]").unwrap()
    })
}

/// Extracts wikilink targets from markdown content.
///
/// # Arguments
/// * `content` - The markdown content to parse
///
/// # Returns
/// A vector of link targets (without display text or headers)
///
/// # Examples
/// ```
/// use buildscale::fs::parsers::extract_links;
///
/// let content = "See [[other note]] and [[third|with display]]";
/// let links = extract_links(content);
/// assert_eq!(links, vec!["other note", "third"]);
/// ```
pub fn extract_links(content: &str) -> Vec<String> {
    wikilink_regex()
        .captures_iter(content)
        .map(|c| c[1].trim().to_string())
        .collect()
}

/// A wikilink with optional display text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wikilink {
    /// The link target (note name)
    pub target: String,
    /// Optional display text
    pub display: Option<String>,
}

/// Extracts wikilinks with their display text from markdown content.
///
/// # Arguments
/// * `content` - The markdown content to parse
///
/// # Returns
/// A vector of Wikilink structs with target and optional display text
pub fn extract_links_with_display(content: &str) -> Vec<Wikilink> {
    wikilink_full_regex()
        .captures_iter(content)
        .map(|c| Wikilink {
            target: c[1].trim().to_string(),
            display: c.get(2).map(|m| m.as_str().trim().to_string()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_link() {
        let content = "This links to [[other]]";
        let links = extract_links(content);
        assert_eq!(links, vec!["other"]);
    }

    #[test]
    fn test_link_with_display() {
        let content = "See [[note|display text]] here";
        let links = extract_links(content);
        assert_eq!(links, vec!["note"]);
    }

    #[test]
    fn test_link_with_header() {
        let content = "See [[note#section]] here";
        let links = extract_links(content);
        assert_eq!(links, vec!["note"]);
    }

    #[test]
    fn test_multiple_links() {
        let content = "Links: [[one]], [[two]], [[three|display]]";
        let links = extract_links(content);
        assert_eq!(links, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_extract_with_display() {
        let content = "[[simple]] and [[with|display]]";
        let links = extract_links_with_display(content);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "simple");
        assert_eq!(links[0].display, None);
        assert_eq!(links[1].target, "with");
        assert_eq!(links[1].display, Some("display".to_string()));
    }
}
