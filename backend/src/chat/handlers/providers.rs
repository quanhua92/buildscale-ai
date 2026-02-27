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
    models::ai_models::AiModel,
    queries::ai_models::{get_models_by_provider, get_workspace_models_by_provider},
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
    /// Whether this is the default model for this workspace
    pub is_default: bool,
    /// Whether this model is free to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_free: Option<bool>,
}

/// Provider configuration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersResponse {
    /// Configured providers with their available models
    pub providers: Vec<ProviderInfo>,
    /// Default provider identifier
    pub default_provider: String,
}

/// Build providers response for global or workspace-scoped models
async fn build_providers_response(
    pool: &sqlx::PgPool,
    ai_config: &crate::config::AiConfig,
    workspace_id: Option<Uuid>,
) -> Result<ProvidersResponse> {
    let default_provider = ai_config.providers.default_provider.clone();
    let default_model = ai_config.providers.default_model.clone();

    // Parse default model identifier
    let (default_provider_for_model, default_model_name) = if default_model.contains(':') {
        let parts: Vec<&str> = default_model.splitn(2, ':').collect();
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (default_provider.clone(), default_model.clone())
    };

    let openai_configured = ai_config.providers.openai.is_some();
    let openrouter_configured = ai_config.providers.openrouter.is_some();
    let mut providers = Vec::new();

    // Helper to build provider info
    let build_provider_info = |provider_name: &str, display_name: &str, models: Vec<AiModel>| {
        let model_infos: Vec<ModelInfo> = models
            .into_iter()
            .map(|m: AiModel| {
                let is_default = default_provider_for_model == provider_name
                    && default_model_name == m.model_name;
                ModelInfo {
                    id: format!("{}:{}", provider_name, m.model_name),
                    provider: provider_name.to_string(),
                    model: m.model_name,
                    display_name: m.display_name,
                    description: m.description,
                    context_window: m.context_window,
                    is_default,
                    is_free: Some(m.is_free),
                }
            })
            .collect();

        ProviderInfo {
            provider: provider_name.to_string(),
            display_name: display_name.to_string(),
            configured: true,
            models: model_infos,
        }
    };

    if openai_configured {
        let models = match workspace_id {
            Some(ws_id) => get_workspace_models_by_provider(pool, ws_id, "openai").await.unwrap_or_default(),
            None => get_models_by_provider(pool, "openai").await.unwrap_or_default(),
        };
        providers.push(build_provider_info("openai", "OpenAI", models));
    }

    if openrouter_configured {
        let models = match workspace_id {
            Some(ws_id) => get_workspace_models_by_provider(pool, ws_id, "openrouter").await.unwrap_or_default(),
            None => get_models_by_provider(pool, "openrouter").await.unwrap_or_default(),
        };
        providers.push(build_provider_info("openrouter", "OpenRouter", models));
    }

    if providers.is_empty() {
        return Err(Error::Internal("No AI providers configured".to_string()));
    }

    Ok(ProvidersResponse { providers, default_provider })
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
    let response = build_providers_response(&state.pool, &state.config.ai, None).await?;
    Ok(Json(response))
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
    axum::extract::Path(workspace_id): axum::extract::Path<Uuid>,
) -> Result<Json<ProvidersResponse>> {
    let response = build_providers_response(&state.pool, &state.config.ai, Some(workspace_id)).await?;
    Ok(Json(response))
}
