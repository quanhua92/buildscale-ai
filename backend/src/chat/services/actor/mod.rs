//! Chat actor module - Manages AI agent lifecycle and interactions

// Include the main actor implementation
mod actor;
pub mod constants;
pub mod interaction;
pub mod session;
pub mod state;
pub mod state_machine;
pub mod stream;

// Re-export public types for backward compatibility
pub use actor::{ChatActor, ChatActorArgs};
pub use interaction::ProcessorContext;
