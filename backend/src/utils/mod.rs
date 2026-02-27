//! Utility modules for BuildScale

pub mod plan_namer;
pub mod frontmatter;
pub mod memory_metadata;
pub mod document_metadata;
pub mod string;
pub mod yaml_frontmatter;

pub use plan_namer::generate_plan_name;
pub use frontmatter::{PlanMetadata, PlanStatus};
pub use memory_metadata::{
    generate_memory_path, parse_memory_path,
    MemoryMetadata, MemoryScope,
};
pub use document_metadata::DocumentMetadata;
pub use string::{safe_preview, truncate_safe, MAX_PREVIEW_LEN};
pub use yaml_frontmatter::{parse_yaml_frontmatter, prepend_yaml_frontmatter};
