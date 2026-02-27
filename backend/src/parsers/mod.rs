//! Parsers for Obsidian-style content extraction
//!
//! This module provides parsers for extracting links and tags from markdown content,
//! following Obsidian's conventions.
//!
//! Note: The actual implementations are now in the `fs::parsers` module.
//! This module re-exports them for backward compatibility.

// Re-export parsers from fs module
pub use crate::fs::parsers::{extract_links, extract_links_with_display, extract_tags, Wikilink};
