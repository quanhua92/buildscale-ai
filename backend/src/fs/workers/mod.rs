//! Background workers for file system indexing
//!
//! This module provides background workers that maintain secondary indexes
//! (links and tags) when files are created or modified.

mod link_indexer;
mod tag_indexer;

pub use link_indexer::link_indexer_worker;
pub use tag_indexer::tag_indexer_worker;
