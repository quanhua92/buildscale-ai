-- Add ai_provider_override column to workspaces table
-- This allows per-workspace provider configuration override

ALTER TABLE workspaces
ADD COLUMN IF NOT EXISTS ai_provider_override TEXT;

-- Add comment for documentation
COMMENT ON COLUMN workspaces.ai_provider_override IS 'Optional per-workspace AI provider override (e.g., "openai", "openrouter"). If NULL, uses global default provider.';
