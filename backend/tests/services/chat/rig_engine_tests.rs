//! Tests for RigService multi-provider functionality

use buildscale::config::{AiConfig, OpenAIConfig, OpenRouterConfig, ProviderConfig};
use buildscale::providers::{AiProvider, ModelIdentifier};
use buildscale::services::chat::rig_engine::RigService;
use secrecy::SecretString;
use std::str::FromStr;

#[test]
fn test_rig_service_from_config_openai_only() {
    // Test creating RigService with only OpenAI provider
    let api_key = SecretString::new("test-openai-key".to_string().into());

    let mut config = AiConfig::default();
    config.providers = ProviderConfig {
        openai: Some(OpenAIConfig {
            api_key: api_key.clone(),
            base_url: None,
            enable_reasoning_summaries: false,
            reasoning_effort: "low".to_string(),
        }),
        openrouter: None,
        default_provider: "openai".to_string(),
        default_model: "openai:gpt-5-mini".to_string(),
    };

    let rig_service = RigService::from_config(&config);
    assert!(
        rig_service.is_ok(),
        "Should create RigService with OpenAI only"
    );

    let rig_service = rig_service.unwrap();
    assert!(rig_service.is_provider_configured(AiProvider::OpenAi));
    assert!(!rig_service.is_provider_configured(AiProvider::OpenRouter));
    assert_eq!(rig_service.default_provider(), AiProvider::OpenAi);
}

#[test]
fn test_rig_service_from_config_openrouter_only() {
    // Test creating RigService with only OpenRouter provider
    let api_key = SecretString::new("test-openrouter-key".to_string().into());

    let mut config = AiConfig::default();
    config.providers = ProviderConfig {
        openai: None,
        openrouter: Some(OpenRouterConfig {
            api_key,
            base_url: None,
        }),
        default_provider: "openrouter".to_string(),
        default_model: "openrouter:anthropic/claude-3.5-sonnet".to_string(),
    };

    let rig_service = RigService::from_config(&config);
    assert!(
        rig_service.is_ok(),
        "Should create RigService with OpenRouter only"
    );

    let rig_service = rig_service.unwrap();
    assert!(!rig_service.is_provider_configured(AiProvider::OpenAi));
    assert!(rig_service.is_provider_configured(AiProvider::OpenRouter));
    assert_eq!(rig_service.default_provider(), AiProvider::OpenRouter);
}

#[test]
fn test_rig_service_from_config_both_providers() {
    // Test creating RigService with both providers
    let openai_key = SecretString::new("test-openai-key".to_string().into());
    let openrouter_key = SecretString::new("test-openrouter-key".to_string().into());

    let mut config = AiConfig::default();
    config.providers = ProviderConfig {
        openai: Some(OpenAIConfig {
            api_key: openai_key,
            base_url: None,
            enable_reasoning_summaries: true,
            reasoning_effort: "high".to_string(),
        }),
        openrouter: Some(OpenRouterConfig {
            api_key: openrouter_key,
            base_url: None,
        }),
        default_provider: "openai".to_string(),
        default_model: "openai:gpt-5-mini".to_string(),
    };

    let rig_service = RigService::from_config(&config);
    assert!(
        rig_service.is_ok(),
        "Should create RigService with both providers"
    );

    let rig_service = rig_service.unwrap();
    assert!(rig_service.is_provider_configured(AiProvider::OpenAi));
    assert!(rig_service.is_provider_configured(AiProvider::OpenRouter));

    let providers = rig_service.configured_providers();
    assert_eq!(providers.len(), 2);
    assert!(providers.contains(&AiProvider::OpenAi));
    assert!(providers.contains(&AiProvider::OpenRouter));
}

#[test]
fn test_rig_service_from_config_no_providers() {
    // Test that creating RigService without any providers fails
    let mut config = AiConfig::default();
    config.providers = ProviderConfig {
        openai: None,
        openrouter: None,
        default_provider: "openai".to_string(),
        default_model: "openai:gpt-5-mini".to_string(),
    };

    let rig_service = RigService::from_config(&config);
    assert!(
        rig_service.is_err(),
        "Should fail when no providers configured"
    );

    let err = rig_service.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("No AI providers configured"),
        "Error should mention no providers configured"
    );
}

#[test]
fn test_rig_service_from_config_invalid_default_provider() {
    // Test that creating RigService with invalid default provider fails
    let api_key = SecretString::new("test-openai-key".to_string().into());

    let mut config = AiConfig::default();
    config.providers = ProviderConfig {
        openai: Some(OpenAIConfig {
            api_key,
            base_url: None,
            enable_reasoning_summaries: false,
            reasoning_effort: "low".to_string(),
        }),
        openrouter: None,
        default_provider: "invalid-provider".to_string(),
        default_model: "openai:gpt-5-mini".to_string(),
    };

    let rig_service = RigService::from_config(&config);
    assert!(
        rig_service.is_err(),
        "Should fail with invalid default provider"
    );

    let err = rig_service.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Invalid default provider"),
        "Error should mention invalid default provider"
    );
}

#[test]
fn test_rig_service_from_config_default_provider_not_configured() {
    // Test that creating RigService with default provider that's not configured fails
    let api_key = SecretString::new("test-openrouter-key".to_string().into());

    let mut config = AiConfig::default();
    config.providers = ProviderConfig {
        openai: None,
        openrouter: Some(OpenRouterConfig {
            api_key,
            base_url: None,
        }),
        default_provider: "openai".to_string(), // Default is OpenAI but only OpenRouter is configured
        default_model: "openai:gpt-5-mini".to_string(),
    };

    let rig_service = RigService::from_config(&config);
    assert!(
        rig_service.is_err(),
        "Should fail when default provider is not configured"
    );

    let err = rig_service.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("OpenAI is not configured"),
        "Error should mention OpenAI is not configured"
    );
}

#[test]
#[allow(deprecated)]
fn test_rig_service_new_backward_compatibility() {
    // Test that the deprecated new() method still works
    let rig_service = RigService::new("test-api-key");
    assert!(rig_service.is_provider_configured(AiProvider::OpenAi));
    assert!(!rig_service.is_provider_configured(AiProvider::OpenRouter));
    assert_eq!(rig_service.default_provider(), AiProvider::OpenAi);
}

#[test]
fn test_rig_service_dummy() {
    // Test that the dummy() method creates a valid service
    let rig_service = RigService::dummy();
    assert!(rig_service.is_provider_configured(AiProvider::OpenAi));
    assert!(!rig_service.is_provider_configured(AiProvider::OpenRouter));
    assert_eq!(rig_service.default_provider(), AiProvider::OpenAi);
}

#[test]
fn test_model_identifier_parse_openai() {
    // Test parsing OpenAI model identifier
    let model = ModelIdentifier::parse("openai:gpt-4o", AiProvider::OpenAi).unwrap();
    assert_eq!(model.provider, AiProvider::OpenAi);
    assert_eq!(model.model, "gpt-4o");
}

#[test]
fn test_model_identifier_parse_openrouter() {
    // Test parsing OpenRouter model identifier
    let model =
        ModelIdentifier::parse("openrouter:anthropic/claude-3.5-sonnet", AiProvider::OpenAi)
            .unwrap();
    assert_eq!(model.provider, AiProvider::OpenRouter);
    assert_eq!(model.model, "anthropic/claude-3.5-sonnet");
}

#[test]
fn test_model_identifier_parse_legacy_openai_default() {
    // Test parsing legacy format with OpenAI as default
    let model = ModelIdentifier::parse("gpt-4o", AiProvider::OpenAi).unwrap();
    assert_eq!(model.provider, AiProvider::OpenAi);
    assert_eq!(model.model, "gpt-4o");
}

#[test]
fn test_model_identifier_parse_legacy_openrouter_default() {
    // Test parsing legacy format with OpenRouter as default
    let model = ModelIdentifier::parse("deepseek-chat", AiProvider::OpenRouter).unwrap();
    assert_eq!(model.provider, AiProvider::OpenRouter);
    assert_eq!(model.model, "deepseek-chat");
}

#[test]
fn test_model_identifier_parse_multiple_colons() {
    // Test that multiple colons are handled correctly
    // The model name can contain colons (e.g., version numbers)
    let model = ModelIdentifier::parse("openai:model:v2", AiProvider::OpenAi).unwrap();
    assert_eq!(model.provider, AiProvider::OpenAi);
    assert_eq!(model.model, "model:v2");
}

#[test]
fn test_model_identifier_parse_invalid_provider() {
    // Test parsing with invalid provider name
    let result = ModelIdentifier::parse("unknown:model", AiProvider::OpenAi);
    assert!(result.is_err(), "Should fail with unknown provider");
}

#[test]
fn test_model_identifier_to_string() {
    // Test converting model identifier to full string format
    let model = ModelIdentifier {
        provider: AiProvider::OpenAi,
        model: "gpt-4o".to_string(),
    };
    assert_eq!(model.to_string(), "openai:gpt-4o");

    let model = ModelIdentifier {
        provider: AiProvider::OpenRouter,
        model: "deepseek-chat".to_string(),
    };
    assert_eq!(model.to_string(), "openrouter:deepseek-chat");
}

#[test]
fn test_model_identifier_to_legacy_string() {
    // Test converting model identifier to legacy format
    let model = ModelIdentifier {
        provider: AiProvider::OpenAi,
        model: "gpt-4o".to_string(),
    };
    assert_eq!(model.to_legacy_string(), "gpt-4o");

    let model = ModelIdentifier {
        provider: AiProvider::OpenRouter,
        model: "anthropic/claude-3.5-sonnet".to_string(),
    };
    assert_eq!(model.to_legacy_string(), "anthropic/claude-3.5-sonnet");
}

#[test]
fn test_ai_provider_from_str() {
    // Test parsing provider from string
    assert_eq!(AiProvider::from_str("openai").unwrap(), AiProvider::OpenAi);
    assert_eq!(AiProvider::from_str("OpenAI").unwrap(), AiProvider::OpenAi);
    assert_eq!(AiProvider::from_str("OPENAI").unwrap(), AiProvider::OpenAi);
    assert_eq!(
        AiProvider::from_str("openrouter").unwrap(),
        AiProvider::OpenRouter
    );
    assert_eq!(
        AiProvider::from_str("OpenRouter").unwrap(),
        AiProvider::OpenRouter
    );

    let result = AiProvider::from_str("unknown");
    assert!(result.is_err(), "Should fail with unknown provider");
}

#[test]
fn test_ai_provider_display() {
    // Test converting provider to string
    assert_eq!(AiProvider::OpenAi.to_string(), "openai");
    assert_eq!(AiProvider::OpenRouter.to_string(), "openrouter");
}

#[test]
fn test_ai_provider_as_str() {
    // Test provider as_str method
    assert_eq!(AiProvider::OpenAi.as_str(), "openai");
    assert_eq!(AiProvider::OpenRouter.as_str(), "openrouter");
}

#[test]
fn test_rig_service_configured_providers_empty() {
    // Test configured_providers when only OpenAI is configured
    let rig_service = RigService::dummy();
    let providers = rig_service.configured_providers();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0], AiProvider::OpenAi);
}

#[test]
fn test_model_identifier_with_complex_model_names() {
    // Test parsing complex model names (with slashes, dots, etc.)
    let model =
        ModelIdentifier::parse("openrouter:anthropic/claude-3.5-sonnet", AiProvider::OpenAi)
            .unwrap();
    assert_eq!(model.provider, AiProvider::OpenRouter);
    assert_eq!(model.model, "anthropic/claude-3.5-sonnet");

    let model = ModelIdentifier::parse(
        "openrouter:microsoft/phi-3-medium-128k-instruct",
        AiProvider::OpenAi,
    )
    .unwrap();
    assert_eq!(model.provider, AiProvider::OpenRouter);
    assert_eq!(model.model, "microsoft/phi-3-medium-128k-instruct");
}

#[test]
fn test_model_identifier_empty_model() {
    // Test parsing with empty model name after colon
    let model = ModelIdentifier::parse("openrouter:", AiProvider::OpenAi).unwrap();
    assert_eq!(model.provider, AiProvider::OpenRouter);
    assert_eq!(model.model, "");
}
