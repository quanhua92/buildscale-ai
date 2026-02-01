//! OpenRouter provider implementation (OpenAI-compatible)

use rig::providers::openrouter::Client;
use secrecy::{ExposeSecret, SecretString};
use std::fmt;

/// OpenRouter provider (OpenAI-compatible)
///
/// OpenRouter provides access to multiple models through a unified API.
/// It's OpenAI-compatible, so we can use similar patterns.
pub struct OpenRouterProvider {
    client: Client,
}

impl fmt::Debug for OpenRouterProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenRouterProvider")
            .field("client", &"<OpenRouter Client>")
            .finish()
    }
}

impl OpenRouterProvider {
    /// Create a new OpenRouter provider
    pub fn new(api_key: &SecretString, base_url: Option<&str>) -> Self {
        let client = if let Some(url) = base_url {
            tracing::info!(
                base_url = %url,
                "Creating OpenRouter provider with custom base URL"
            );
            Client::builder()
                .api_key(api_key.expose_secret())
                .base_url(url)
                .build()
                .expect("Failed to create OpenRouter client with custom base URL")
        } else {
            tracing::info!("Creating OpenRouter provider with default base URL");
            Client::new(api_key.expose_secret())
                .expect("Failed to create OpenRouter client")
        };

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

    #[test]
    fn test_openrouter_provider_with_custom_base_url() {
        let api_key = SecretString::new("test-key".to_string().into());
        let custom_url = "https://custom.openrouter.com/api/v1";
        let _provider = OpenRouterProvider::new(&api_key, Some(custom_url));
        // Test passes if provider was created without panicking
        // In real usage, this would connect to the custom URL
    }
}
