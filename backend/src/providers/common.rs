//! Common provider types and traits for OpenAI-compatible providers

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Supported AI providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    OpenAi,
    OpenRouter,
}

impl AiProvider {
    /// Returns the provider identifier string
    pub fn as_str(&self) -> &'static str {
        match self {
            AiProvider::OpenAi => "openai",
            AiProvider::OpenRouter => "openrouter",
        }
    }
}

impl FromStr for AiProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(AiProvider::OpenAi),
            "openrouter" => Ok(AiProvider::OpenRouter),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

impl std::fmt::Display for AiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Parsed model identifier with provider and model name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelIdentifier {
    pub provider: AiProvider,
    pub model: String,
}

impl ModelIdentifier {
    /// Parse a model string (supports both "provider:model" and legacy "model" formats)
    pub fn parse(input: &str, default_provider: AiProvider) -> Result<Self, String> {
        if input.contains(':') {
            // New format: "provider:model"
            let parts: Vec<&str> = input.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid model format: {}", input));
            }
            let provider = AiProvider::from_str(parts[0])?;
            let model = parts[1].to_string();
            Ok(ModelIdentifier { provider, model })
        } else {
            // Legacy format: "model" (use default provider)
            Ok(ModelIdentifier {
                provider: default_provider,
                model: input.to_string(),
            })
        }
    }

    /// Convert to full string format
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.provider.as_str(), self.model)
    }

    /// Get legacy format (model name only)
    pub fn to_legacy_string(&self) -> String {
        self.model.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_new_format_openai() {
        let model = ModelIdentifier::parse("openai:gpt-4o", AiProvider::OpenAi).unwrap();
        assert_eq!(model.provider, AiProvider::OpenAi);
        assert_eq!(model.model, "gpt-4o");
    }

    #[test]
    fn test_parse_new_format_openrouter() {
        let model = ModelIdentifier::parse("openrouter:anthropic/claude-3.5-sonnet", AiProvider::OpenAi).unwrap();
        assert_eq!(model.provider, AiProvider::OpenRouter);
        assert_eq!(model.model, "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_parse_legacy_format() {
        let model = ModelIdentifier::parse("gpt-4o", AiProvider::OpenAi).unwrap();
        assert_eq!(model.provider, AiProvider::OpenAi);
        assert_eq!(model.model, "gpt-4o");
    }

    #[test]
    fn test_invalid_provider() {
        let result = ModelIdentifier::parse("unknown:model", AiProvider::OpenAi);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_string() {
        let model = ModelIdentifier {
            provider: AiProvider::OpenAi,
            model: "gpt-4o".to_string(),
        };
        assert_eq!(model.to_string(), "openai:gpt-4o");
    }

    #[test]
    fn test_to_legacy_string() {
        let model = ModelIdentifier {
            provider: AiProvider::OpenRouter,
            model: "anthropic/claude-3.5-sonnet".to_string(),
        };
        assert_eq!(model.to_legacy_string(), "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!(AiProvider::from_str("openai").unwrap(), AiProvider::OpenAi);
        assert_eq!(AiProvider::from_str("OpenAI").unwrap(), AiProvider::OpenAi);
        assert_eq!(AiProvider::from_str("OPENAI").unwrap(), AiProvider::OpenAi);
        assert_eq!(AiProvider::from_str("openrouter").unwrap(), AiProvider::OpenRouter);
        assert!(AiProvider::from_str("unknown").is_err());
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(AiProvider::OpenAi.to_string(), "openai");
        assert_eq!(AiProvider::OpenRouter.to_string(), "openrouter");
    }
}
