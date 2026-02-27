//! Chat module for BuildScale AI
//!
//! Organized into layers:
//! - `models/` - Chat message and session data structures
//! - `queries/` - Database operations
//! - `services/` - Core services (actor, engine, state machine)
//! - `handlers/` - HTTP API endpoints

pub mod models;
pub mod queries;
pub mod services;
pub mod handlers;

// Re-export commonly used types
pub use models::{ChatMessage, NewChatMessage, ChatAttachment};
pub use services::{ChatService, RigService, BuiltContext, ChatFrontmatter};
pub use services::actor::ChatActor;
pub use handlers::{create_chat, get_chat, post_chat_message, list_chats, get_providers};
