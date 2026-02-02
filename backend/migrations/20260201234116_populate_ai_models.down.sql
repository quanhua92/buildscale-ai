-- Rollback populate_ai_models migration
-- This removes all inserted models

-- Remove all models inserted by this migration
DELETE FROM ai_models WHERE provider IN ('openai', 'openrouter');
