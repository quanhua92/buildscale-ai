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

use std::fmt;

/// Our unified agent type that wraps either OpenAI or OpenRouter agents
///
/// This allows the rest of the system to work with a single Agent type
/// while each provider handles its own agent building internally.
pub enum Agent {
    OpenAI(rig::agent::Agent<rig::providers::openai::responses_api::ResponsesCompletionModel>),
    OpenRouter(rig::agent::Agent<rig::providers::openrouter::CompletionModel>),
}

// Manually implement Debug since rig::agent::Agent doesn't implement it
impl fmt::Debug for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Agent::OpenAI(_) => f.debug_tuple("Agent::OpenAI").field(&"<OpenAI Agent>").finish(),
            Agent::OpenRouter(_) => f.debug_tuple("Agent::OpenRouter").field(&"<OpenRouter Agent>").finish(),
        }
    }
}

// Implement Clone for Agent since agents are cached in ChatActor
impl Clone for Agent {
    fn clone(&self) -> Self {
        match self {
            Agent::OpenAI(agent) => Agent::OpenAI(agent.clone()),
            Agent::OpenRouter(agent) => Agent::OpenRouter(agent.clone()),
        }
    }
}
