//! HTTP handlers for BuildScale API
//!
//! Handlers are organized by API domain:
//! - `auth/` - Authentication endpoints
//! - `workspace/` - Workspace and member management
//! - `file/` - File system and tool operations
//! - `chat/` - Chat and AI agent endpoints

//! - `agent/` - AI Agent Session endpoints

// Subdirectory modules (layered structure)
pub mod auth;
pub mod workspace;
pub mod file;
pub mod system;

// Re-export all handlers for backward compatibility
pub use auth::*;
pub use workspace::*;
pub use file::*;
pub use system::*;

// Re-export chat handlers from chat module
pub use crate::chat::handlers::chat::*;
pub use crate::chat::handlers::message::*;
pub use crate::chat::handlers::providers::*;

// Re-export agent handlers from agent module
pub use crate::agent::handlers::*;
