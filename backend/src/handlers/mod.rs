//! HTTP handlers for BuildScale API
//!
//! Handlers are organized by API domain:
//! - `auth/` - Authentication endpoints
//! - `workspace/` - Workspace and member management
//! - `file/` - File system and tool operations
//! - `chat/` - Chat and AI agent endpoints

// Subdirectory modules (layered structure)
pub mod auth;
pub mod workspace;
pub mod file;
pub mod chat;

// Health check stays at root level
pub mod health;

// Re-export all handlers for backward compatibility
pub use auth::*;
pub use workspace::*;
pub use file::*;
pub use chat::*;
pub use health::*;
