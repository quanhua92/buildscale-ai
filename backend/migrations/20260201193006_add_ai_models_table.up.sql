-- Add AI models table to store all models from all providers
CREATE TABLE IF NOT EXISTS ai_models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider TEXT NOT NULL, -- 'openai', 'openrouter', etc.
    model_name TEXT NOT NULL, -- e.g., 'gpt-4o', 'anthropic/claude-3.5-sonnet'
    display_name TEXT NOT NULL, -- e.g., 'GPT-4o', 'Claude 3.5 Sonnet'
    description TEXT,
    context_window INTEGER DEFAULT 128000,
    is_enabled BOOLEAN NOT NULL DEFAULT false, -- Disable unwanted models by default
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider, model_name)
);

-- Add index for faster lookups by provider
CREATE INDEX idx_ai_models_provider ON ai_models(provider);
CREATE INDEX idx_ai_models_enabled ON ai_models(is_enabled) WHERE is_enabled = true;

-- Add workspace-model mapping table to control which workspaces can use which models
CREATE TABLE IF NOT EXISTS workspace_ai_models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    model_id UUID NOT NULL REFERENCES ai_models(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'active', -- 'active', 'disabled', 'restricted'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, model_id)
);

-- Add index for faster workspace model lookups
CREATE INDEX idx_workspace_ai_models_workspace ON workspace_ai_models(workspace_id);
CREATE INDEX idx_workspace_ai_models_status ON workspace_ai_models(status);

-- Add comment for documentation
COMMENT ON TABLE ai_models IS 'Stores all available AI models from all providers with global enable/disable control';
COMMENT ON COLUMN ai_models.is_enabled IS 'Global flag to disable unwanted models (e.g., expensive models)';
COMMENT ON TABLE workspace_ai_models IS 'Maps workspaces to models with access control status';
COMMENT ON COLUMN workspace_ai_models.status IS 'Access control status: active, disabled, restricted';
