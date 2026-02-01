# AI Providers

BuildScale supports multiple AI provider integrations with granular control over model availability and workspace access.

## Architecture

### Provider Abstraction

The system uses a modular provider architecture located in `src/providers/`:

- **`common.rs`** - Shared types and model identifier parsing
  - `AiProvider` enum: `OpenAi`, `OpenRouter`
  - `ModelIdentifier` parses `"provider:model"` and legacy `"model"` formats

- **`openai.rs`** - OpenAI provider implementation
  - Supports reasoning summaries for GPT-5 models
  - Configurable reasoning effort: `"low"`, `"medium"`, `"high"`

- **`openrouter.rs`** - OpenRouter provider
  - Provides access to 200+ models through OpenRouter's unified API
  - OpenAI-compatible interface

## Configuration

### Environment Variables

```bash
# OpenAI Provider
BUILDSCALE__AI__PROVIDERS__OPENAI__API_KEY=sk-...
BUILDSCALE__AI__PROVIDERS__OPENAI__ENABLE_REASONING_SUMMARIES=true
BUILDSCALE__AI__PROVIDERS__OPENAI__REASONING_EFFORT=medium

# OpenRouter Provider
BUILDSCALE__AI__PROVIDERS__OPENROUTER__API_KEY=sk-or-...

# Default Provider
BUILDSCALE__AI__PROVIDERS__DEFAULT_PROVIDER=openai

# Legacy (deprecated, auto-migrates to providers.openai.api_key)
BUILDSCALE__AI__OPENAI_API_KEY=sk-...
```

### Configuration Structures

```rust
// src/config.rs
pub struct ProviderConfig {
    pub openai: Option<OpenAIConfig>,
    pub openrouter: Option<OpenRouterConfig>,
    pub default_provider: String, // "openai" or "openrouter"
}

pub struct OpenAIConfig {
    pub api_key: SecretString,
    pub base_url: Option<String>,
    pub enable_reasoning_summaries: bool,
    pub reasoning_effort: String,
}

pub struct OpenRouterConfig {
    pub api_key: SecretString,
    pub base_url: Option<String>,
}
```

## Model Identifier Format

### New Format (Recommended)
```
provider:model-name
```

Examples:
- `openai:gpt-4o`
- `openai:gpt-5-mini`
- `openrouter:anthropic/claude-3.5-sonnet`
- `openrouter:google/gemini-pro-1.5`

### Legacy Format (Backward Compatible)
```
model-name
```

Uses default provider (configured via `BUILDSCALE__AI__PROVIDERS__DEFAULT_PROVIDER`):
- `gpt-4o` → interpreted as `openai:gpt-4o` (if default is openai)

### Parsing Logic

```rust
use crate::providers::ModelIdentifier;

let model = ModelIdentifier::parse("openai:gpt-4o", AiProvider::OpenAi)?;
// Returns: ModelIdentifier { provider: OpenAi, model: "gpt-4o" }

let legacy = ModelIdentifier::parse("gpt-4o", AiProvider::OpenAi)?;
// Returns: ModelIdentifier { provider: OpenAi, model: "gpt-4o" }
```

## Database Schema

### ai_models Table

Stores all available AI models from all providers with global enable/disable control.

```sql
CREATE TABLE ai_models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider TEXT NOT NULL,              -- 'openai', 'openrouter'
    model_name TEXT NOT NULL,            -- 'gpt-4o', 'anthropic/claude-3.5-sonnet'
    display_name TEXT NOT NULL,          -- 'GPT-4o', 'Claude 3.5 Sonnet'
    description TEXT,
    context_window INTEGER DEFAULT 128000,
    is_enabled BOOLEAN NOT NULL DEFAULT false,  -- Globally disable unwanted models
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider, model_name)
);
```

**Key Fields**:
- `is_enabled` - Set to `false` to disable expensive models globally
- `context_window` - Model's token capacity for context
- Unique constraint on `(provider, model_name)`

### workspace_ai_models Table

Controls which workspaces can access which models.

```sql
CREATE TABLE workspace_ai_models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    model_id UUID NOT NULL REFERENCES ai_models(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'active',  -- 'active', 'disabled', 'restricted'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, model_id)
);
```

**Status Values**:
- `active` - Workspace can use this model
- `disabled` - Workspace cannot use this model
- `restricted` - Special access control (e.g., premium models)

## API Usage

### Query Functions

```rust
use crate::queries::ai_models;

// Get all enabled models
let models = get_enabled_models(&pool).await?;

// Get models by provider
let openai_models = get_models_by_provider(&pool, "openai").await?;

// Get models available to a workspace
let workspace_models = get_workspace_enabled_models(&pool, workspace_id).await?;

// Check if workspace can use a specific model
let has_access = check_workspace_model_access(&pool, workspace_id, model_id).await?;

// Grant workspace access to a model
grant_workspace_model_access(&pool, &NewWorkspaceAiModel {
    workspace_id,
    model_id,
    status: ModelAccessStatus::Active.as_str(),
}).await?;

// Revoke workspace access
revoke_workspace_model_access(&pool, workspace_id, model_id).await?;
```

## Use Cases

### 1. Cost Control

Even with an OpenRouter API key that provides access to 200+ models, you can:

```sql
-- Globally disable expensive models
UPDATE ai_models SET is_enabled = false
WHERE model_name LIKE '%claude-3.5%' OR model_name LIKE '%gpt-5%';

-- Enable only cheap models
UPDATE ai_models SET is_enabled = true
WHERE model_name LIKE '%kimi%' OR model_name LIKE '%deepseek%';
```

### 2. Workspace Tier Management

```sql
-- Free tier workspaces get basic models
INSERT INTO workspace_ai_models (workspace_id, model_id, status)
SELECT
    ws.id,
    m.id,
    'active'
FROM workspaces ws
CROSS JOIN ai_models m
WHERE ws.tier = 'free'
  AND m.model_name IN ('deepseek-chat', 'kimi-2.5');

-- Premium tier workspaces get all models
INSERT INTO workspace_ai_models (workspace_id, model_id, status)
SELECT
    ws.id,
    m.id,
    'active'
FROM workspaces ws
CROSS JOIN ai_models m
WHERE ws.tier = 'premium'
  AND m.is_enabled = true;
```

### 3. Security Restrictions

```sql
-- Restrict certain workspaces from using advanced models
UPDATE workspace_ai_models
SET status = 'restricted'
WHERE model_id IN (
    SELECT id FROM ai_models
    WHERE model_name LIKE '%code%' OR model_name LIKE '%advanced%'
)
AND workspace_id IN (
    SELECT id FROM workspaces
    WHERE security_level = 'standard'
);
```

## Migration Strategy

### Legacy Model Strings

The system automatically migrates legacy model strings to the new format:

```sql
-- Convert legacy "gpt-4o" to "openai:gpt-4o"
UPDATE file_versions
SET app_data = jsonb_set(
    app_data,
    '{model}',
    ('openai:' || (app_data->>'model'))::jsonb
)
WHERE app_data ? 'model'
AND (app_data->>'model') NOT LIKE '%:%';
```

### Runtime Migration

In `get_chat_session()`, legacy format is detected and migrated automatically:
- Detects strings without colon (`:`)
- Prepends default provider
- Logs migration warning for monitoring

## Provider Capabilities

### OpenAI
- **Models**: GPT-4o, GPT-4o-mini, GPT-5, GPT-5-mini, GPT-5-nano, GPT-5.1
- **Reasoning**: Supported with `enable_reasoning_summaries`
- **Response Continuity**: `previous_response_id` for conversation continuity
- **Context Window**: Up to 128k tokens

### OpenRouter
- **Models**: 200+ models including:
  - Anthropic: Claude 3.5 Sonnet, Claude 3 Opus
  - Google: Gemini Pro 1.5, Gemini 2.0 Flash
  - Meta: Llama 3.1 70B, Llama 3.1 405B
  - DeepSeek: DeepSeek Chat, DeepSeek Coder
  - Kimi: Kimi 2.5
  - And 190+ more
- **Reasoning**: Not supported (provider-agnostic)
- **Context Window**: Varies by model
- **Advantage**: Single API key for multiple providers

## Error Handling

### Provider-Specific Errors

```rust
pub enum Error {
    // ...
    #[error("AI provider error: {0}")]
    AiProvider(String),

    #[error("Provider {0} not configured")]
    ProviderNotConfigured(String),

    #[error("Invalid model format: {0}")]
    InvalidModelFormat(String),

    #[error("Model {0} not supported by provider {1}")]
    ModelNotSupported(String, String),

    #[error("API key not configured for provider {0}")]
    ApiKeyMissing(String),
}
```

## Best Practices

### 1. Model Registration

When adding new models to the system:

```rust
use crate::queries::ai_models;

let model = create_model(&pool, &NewAiModel {
    provider: "openrouter".to_string(),
    model_name: "deepseek-chat".to_string(),
    display_name: "DeepSeek Chat".to_string(),
    description: Some("Affordable Chinese language model".to_string()),
    context_window: Some(128000),
    is_enabled: true, // Enable by default for cheap models
}).await?;
```

### 2. Workspace Onboarding

When a new workspace is created, grant access to appropriate models:

```rust
// Grant access to all enabled models
let enabled_models = get_enabled_models(&pool).await?;
for model in enabled_models {
    grant_workspace_model_access(&pool, &NewWorkspaceAiModel {
        workspace_id: new_workspace_id,
        model_id: model.id,
        status: ModelAccessStatus::Active.as_str(),
    }).await?;
}
```

### 3. Model Access Validation

Before executing a chat request:

```rust
use crate::queries::ai_models;

let model_id = ModelIdentifier::parse(&session.agent_config.model, default_provider)?;

// Check if model is globally enabled
let model = get_model_by_id(&pool, model_id_from_name).await?;
if !model.is_enabled {
    return Err(Error::ModelDisabled(model.model_name));
}

// Check if workspace has access
let has_access = check_workspace_model_access(&pool, workspace_id, model.id).await?;
if !has_access {
    return Err(Error::Forbidden(format!(
        "Workspace does not have access to model: {}",
        model.model_name
    )));
}
```

## Configuration Examples

### Development Environment

```bash
# .env
BUILDSCALE__AI__PROVIDERS__OPENAI__API_KEY=sk-dev-...
BUILDSCALE__AI__PROVIDERS__OPENROUTER__API_KEY=sk-or-dev-...
BUILDSCALE__AI__PROVIDERS__DEFAULT_PROVIDER=openai
```

### Production Environment

```bash
# Production: Only OpenAI, no OpenRouter
BUILDSCALE__AI__PROVIDERS__OPENAI__API_KEY=sk-prod-...
BUILDSCALE__AI__PROVIDERS__DEFAULT_PROVIDER=openai
```

### Cost-Optimized Environment

```bash
# Use OpenRouter for cheap models only
BUILDSCALE__AI__PROVIDERS__OPENROUTER__API_KEY=sk-or-...
BUILDSCALE__AI__PROVIDERS__DEFAULT_PROVIDER=openrouter

# Disable expensive models in database
UPDATE ai_models SET is_enabled = false
WHERE provider = 'openrouter'
AND (model_name LIKE '%claude%' OR model_name LIKE '%gpt-5%');
```

## Future Enhancements

### Planned Features

1. **Per-Provider Rate Limiting**
   - Track usage per provider
   - Enforce quotas per workspace

2. **Model Capabilities Registry**
   - Vision support flag
   - Function calling support
   - Streaming support

3. **Dynamic Model Discovery**
   - Automatically fetch available models from providers
   - Sync with provider APIs

4. **Usage Analytics**
   - Track model usage per workspace
   - Cost breakdown by provider and model

## Related Documentation

- [RIG_INTEGRATION.md](./RIG_INTEGRATION.md) - How Rig providers are integrated
- [CONFIGURATION.md](./CONFIGURATION.md) - Full configuration reference
- [SERVICES_API_GUIDE.md](./SERVICES_API_GUIDE.md) - API endpoint documentation
