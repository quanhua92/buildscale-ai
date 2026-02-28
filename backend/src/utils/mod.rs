//! Utility modules for BuildScale

pub mod plan_namer;
pub mod memory_metadata;
pub mod string;

pub use plan_namer::generate_plan_name;
pub use memory_metadata::{
    generate_memory_path, parse_memory_path,
    MemoryMetadata, MemoryScope,
};
pub use string::{safe_preview, truncate_safe, MAX_PREVIEW_LEN};

// Re-export file system utilities from fs module
pub use crate::fs::utils::{
    DocumentMetadata, PlanMetadata, PlanStatus,
    parse_yaml_frontmatter, prepend_yaml_frontmatter,
};
