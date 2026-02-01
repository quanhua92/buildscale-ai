//! OpenRouter provider implementation (OpenAI-compatible)

use rig::providers::openrouter::Client;
use secrecy::{ExposeSecret, SecretString};

/// OpenRouter provider (OpenAI-compatible)
///
/// OpenRouter provides access to multiple models through a unified API.
/// It's OpenAI-compatible, so we can use similar patterns.
pub struct OpenRouterProvider {
    client: Client,
}

impl OpenRouterProvider {
    /// Create a new OpenRouter provider
    pub fn new(api_key: &SecretString, _base_url: Option<&str>) -> Self {
        let client = Client::new(api_key.expose_secret())
            .expect("Failed to create OpenRouter client");

        Self { client }
    }

    /// Get a reference to the underlying OpenRouter client
    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openrouter_provider_creation() {
        let api_key = SecretString::new("test-key".to_string().into());
        let _provider = OpenRouterProvider::new(&api_key, None);
        // Test passes if provider was created without panicking
    }
}
