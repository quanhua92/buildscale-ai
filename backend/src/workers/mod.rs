//! Background workers for BuildScale
//!
//! Workers are organized by domain:
//! - `auth/` - Authentication token cleanup
//! - `storage/` - Storage blob cleanup

pub mod auth;
pub mod storage;

// Re-export for backward compatibility
pub use auth::revoked_token_cleanup_worker;
pub use storage::archive_cleanup_worker;
