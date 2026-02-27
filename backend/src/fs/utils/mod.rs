//! File system utilities for document metadata and frontmatter
//!
//! This module provides utilities for working with document metadata
//! and YAML frontmatter in files.

mod document_metadata;
mod frontmatter;
mod yaml_frontmatter;

pub use document_metadata::DocumentMetadata;
pub use frontmatter::{PlanMetadata, PlanStatus};
pub use yaml_frontmatter::{parse_yaml_frontmatter, prepend_yaml_frontmatter};
