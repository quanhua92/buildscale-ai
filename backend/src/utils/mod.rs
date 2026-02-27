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

// Backward compatible module re-exports for code using crate::utils::frontmatter:: etc.
pub mod document_metadata {
    pub use crate::fs::utils::DocumentMetadata;
}

pub mod frontmatter {
    pub use crate::fs::utils::{PlanMetadata, PlanStatus};
}

pub mod yaml_frontmatter {
    pub use crate::fs::utils::{parse_yaml_frontmatter, prepend_yaml_frontmatter};
}
