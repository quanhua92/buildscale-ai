//! Chat actor module - Manages AI agent lifecycle and interactions

// Include the main actor implementation
mod actor_impl;
pub mod constants;
pub mod state;
pub mod session;

// Re-export public types for backward compatibility
pub use actor_impl::{ChatActor, ChatActorArgs};
