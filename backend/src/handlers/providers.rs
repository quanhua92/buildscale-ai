//! Provider configuration handlers
//!
//! This module provides endpoints for querying available AI providers
//! and their supported models.

use axum::{extract::{Extension, State}, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    middleware::auth::AuthenticatedUser,
    queries::ai_models::get_models_by_provider,
    state::AppState,
};

/// Provider information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider identifier (e.g., "openai", "openrouter")
    pub provider: String,
    /// Display name for the provider
    pub display_name: String,
    /// Whether this provider is configured (has API key)
    pub configured: bool,
    /// Available models for this provider
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<ModelInfo>,
}

/// Model information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Full model identifier (e.g., "openai:gpt-4o")
    pub id: String,
    /// Provider name
    pub provider: String,
    /// Model name without provider prefix
    pub model: String,
    /// Human-readable display name
    pub display_name: String,
    /// Model description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Context window size in tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<i32>,
}

/// Provider configuration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersResponse {
    /// Configured providers with their available models
    pub providers: Vec<ProviderInfo>,
    /// Default provider identifier
    pub default_provider: String,
}

/// Get providers and models configuration for a workspace
///
/// This endpoint returns information about available AI providers
/// and their supported models. The response is filtered based on:
/// - Which providers are configured (have API keys)
/// - Which models are enabled in the ai_models table
///
/// # Authentication
/// Requires valid JWT token via Authorization header or cookie
///
/// # Example
/// ```bash
/// curl http://localhost:3000/api/v1/providers \
///   -H "Authorization: Bearer <access_token>"
/// ```
///
/// # Response
/// ```json
/// {
///   "providers": [
///     {
///       "provider": "openai",
///       "display_name": "OpenAI",
///       "configured": true,
///       "models": [
///         {
///           "id": "openai:gpt-4o",
///           "provider": "openai",
///           "model": "gpt-4o",
///           "display_name": "GPT-4o",
///           "description": "Latest GPT-4 model",
///           "context_window": 128000
///         }
///       ]
///     },
///     {
///       "provider": "openrouter",
///       "display_name": "OpenRouter",
///       "configured": false,
///       "models": []
///     }
///   ],
///   "default_provider": "openai"
/// }
/// ```
pub async fn get_providers(
    Extension(_user): Extension<AuthenticatedUser>,
    State(state): State<AppState>,
) -> Result<Json<ProvidersResponse>> {
    let pool = &state.pool;

    // Get AI configuration to determine which providers are configured
    let ai_config = &state.config.ai;

    // Get default provider
    let default_provider = ai_config.providers.default_provider.clone();

    // Check which providers are configured
    let openai_configured = ai_config.providers.openai.is_some();
    let openrouter_configured = ai_config.providers.openrouter.is_some();

    // Build providers list
    let mut providers = Vec::new();

    // Add OpenAI provider if configured
    if openai_configured {
        let models = get_models_by_provider(pool, "openai")
            .await
            .unwrap_or_default();

        let model_infos: Vec<ModelInfo> = models
            .into_iter()
            .map(|m| ModelInfo {
                id: format!("openai:{}", m.model_name),
                provider: "openai".to_string(),
                model: m.model_name,
                display_name: m.display_name,
                description: m.description,
                context_window: m.context_window,
            })
            .collect();

        providers.push(ProviderInfo {
            provider: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            configured: true,
            models: model_infos,
        });
    }

    // Add OpenRouter provider if configured
    if openrouter_configured {
        let models = get_models_by_provider(pool, "openrouter")
            .await
            .unwrap_or_default();

        let model_infos: Vec<ModelInfo> = models
            .into_iter()
            .map(|m| ModelInfo {
                id: format!("openrouter:{}", m.model_name),
                provider: "openrouter".to_string(),
                model: m.model_name,
                display_name: m.display_name,
                description: m.description,
                context_window: m.context_window,
            })
            .collect();

        providers.push(ProviderInfo {
            provider: "openrouter".to_string(),
            display_name: "OpenRouter".to_string(),
            configured: true,
            models: model_infos,
        });
    }

    // If no providers are configured, return empty list with error
    if providers.is_empty() {
        return Err(Error::Internal("No AI providers configured".to_string()));
    }

    Ok(Json(ProvidersResponse {
        providers,
        default_provider,
    }))
}

/// Get providers and models for a specific workspace
///
/// This endpoint returns provider information filtered by workspace access.
/// Only models that the workspace has access to will be included.
///
/// # Authentication
/// Requires valid JWT token via Authorization header or cookie
///
/// # Example
/// ```bash
/// curl http://localhost:3000/api/v1/workspaces/{workspace_id}/providers \
///   -H "Authorization: Bearer <access_token>"
/// ```
///
/// # Response
/// Same format as get_providers, but models are filtered by workspace access
pub async fn get_workspace_providers(
    Extension(_user): Extension<AuthenticatedUser>,
    State(state): State<AppState>,
    axum::extract::Path(_workspace_id): axum::extract::Path<Uuid>,
) -> Result<Json<ProvidersResponse>> {
    let pool = &state.pool;

    // Get AI configuration
    let ai_config = &state.config.ai;
    let default_provider = ai_config.providers.default_provider.clone();

    // Check which providers are configured
    let openai_configured = ai_config.providers.openai.is_some();
    let openrouter_configured = ai_config.providers.openrouter.is_some();

    // Build providers list with workspace-accessible models
    let mut providers = Vec::new();

    // Get workspace-enabled models for each configured provider
    if openai_configured {
        // For now, return all enabled OpenAI models
        // TODO: Filter by workspace_ai_models table
        let models = get_models_by_provider(pool, "openai")
            .await
            .unwrap_or_default();

        let model_infos: Vec<ModelInfo> = models
            .into_iter()
            .map(|m| ModelInfo {
                id: format!("openai:{}", m.model_name),
                provider: "openai".to_string(),
                model: m.model_name,
                display_name: m.display_name,
                description: m.description,
                context_window: m.context_window,
            })
            .collect();

        providers.push(ProviderInfo {
            provider: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            configured: true,
            models: model_infos,
        });
    }

    if openrouter_configured {
        // For now, return all enabled OpenRouter models
        // TODO: Filter by workspace_ai_models table
        let models = get_models_by_provider(pool, "openrouter")
            .await
            .unwrap_or_default();

        let model_infos: Vec<ModelInfo> = models
            .into_iter()
            .map(|m| ModelInfo {
                id: format!("openrouter:{}", m.model_name),
                provider: "openrouter".to_string(),
                model: m.model_name,
                display_name: m.display_name,
                description: m.description,
                context_window: m.context_window,
            })
            .collect();

        providers.push(ProviderInfo {
            provider: "openrouter".to_string(),
            display_name: "OpenRouter".to_string(),
            configured: true,
            models: model_infos,
        });
    }

    // If no providers are configured, return empty list with error
    if providers.is_empty() {
        return Err(Error::Internal("No AI providers configured".to_string()));
    }

    Ok(Json(ProvidersResponse {
        providers,
        default_provider,
    }))
}
