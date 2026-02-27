//! Chat and AI agent handlers
//!
//! Re-exports handlers from the new agent and chat modules for backward compatibility.

// Re-export agent handlers from agent module
pub use crate::agent::handlers::*;

// Re-export chat handlers from chat module
pub use crate::chat::handlers::*;
