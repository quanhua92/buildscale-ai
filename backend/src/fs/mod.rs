//! Core file system module (Layer 1)
//!
//! This module provides the foundation for all file operations.
//! It is consumed by:
//! - REST handlers (src/handlers/files.rs)
//! - AI tools (src/tools/)
//!
//! ## Architecture
//!
//! ```text
//! Layer 4: workflow/pipeline  (future - orchestrates everything)
//! Layer 3: agents             (uses tools)        <- src/agents/
//! Layer 2: tools              (AI interface)      <- src/tools/
//! Layer 2: handlers           (REST interface)    <- src/handlers/
//! Layer 1: fs                 (core)              <- src/fs/        <-- YOU ARE HERE
//! ```
//!
//! ## Design Principles
//!
//! 1. **Single Source of Truth**: This module is the only place for file system logic
//! 2. **Interface Independence**: REST and AI can evolve independently
//! 3. **Clear Dependencies**: This layer only depends on database, disk storage, and stdlib
//! 4. **Testability**: Can be tested without HTTP or AI concerns

pub mod models;
pub mod queries;
pub mod services;
pub mod storage;
pub mod parsers;
pub mod workers;
pub mod utils;

// Re-exports for convenience
pub use models::*;
pub use storage::FileStorageService;
pub use parsers::{extract_links, extract_links_with_display, extract_tags, Wikilink};
pub use workers::{link_indexer_worker, tag_indexer_worker};
