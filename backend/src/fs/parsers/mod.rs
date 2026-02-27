//! Parsers for Obsidian-style content extraction
//!
//! This module provides parsers for extracting links and tags from markdown content,
//! following Obsidian's conventions.

mod links;
mod tags;

pub use links::{extract_links, extract_links_with_display, Wikilink};
pub use tags::extract_tags;
