//! OpenAI provider implementation with reasoning support

use rig::providers::openai::Client;
use secrecy::{ExposeSecret, SecretString};

/// OpenAI provider with reasoning support
pub struct OpenAiProvider {
    client: Client,
    enable_reasoning: bool,
    reasoning_effort: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: &SecretString, _base_url: Option<&str>) -> Self {
        let client = Client::new(api_key.expose_secret())
            .expect("Failed to create OpenAI client");

        Self {
            client,
            enable_reasoning: false,
            reasoning_effort: "low".to_string(),
        }
    }

    /// Enable reasoning summaries for GPT-5 models
    pub fn with_reasoning(mut self, enable: bool, effort: String) -> Self {
        self.enable_reasoning = enable;
        self.reasoning_effort = effort;
        self
    }

    /// Get a reference to the underlying OpenAI client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Check if reasoning is enabled
    pub fn is_reasoning_enabled(&self) -> bool {
        self.enable_reasoning
    }

    /// Get the reasoning effort level
    pub fn reasoning_effort(&self) -> &str {
        &self.reasoning_effort
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_creation() {
        let api_key = SecretString::new("test-key".to_string().into());
        let provider = OpenAiProvider::new(&api_key, None);
        assert!(!provider.is_reasoning_enabled());
        assert_eq!(provider.reasoning_effort(), "low");
    }

    #[test]
    fn test_openai_provider_with_reasoning() {
        let api_key = SecretString::new("test-key".to_string().into());
        let provider = OpenAiProvider::new(&api_key, None).with_reasoning(true, "high".to_string());
        assert!(provider.is_reasoning_enabled());
        assert_eq!(provider.reasoning_effort(), "high");
    }
}
