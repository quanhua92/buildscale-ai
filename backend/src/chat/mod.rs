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

// Re-export handlers
pub use handlers::chat::{create_chat, get_chat, post_chat_message};
pub use handlers::message::list_chats;
pub use handlers::providers::get_providers;
