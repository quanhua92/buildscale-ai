//! Multi-provider AI support
//!
//! This module provides abstraction for multiple AI providers (OpenAI, OpenRouter)
//! with a common interface for OpenAI-compatible providers.

pub mod common;
pub mod openai;
pub mod openrouter;

// Re-export common types
pub use common::{AiProvider, ModelIdentifier};

// Re-export providers
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
