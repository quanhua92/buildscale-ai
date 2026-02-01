-- Migrate legacy model strings to new provider:model format
-- This migration converts existing model strings in file_versions app_data
-- from legacy format ("gpt-4o") to new format ("openai:gpt-4o")

-- Update file_versions app_data to add provider prefix to legacy model strings
-- Only affects rows where:
-- 1. app_data has a 'model' field
-- 2. model field does not already contain a colon (provider prefix)
UPDATE file_versions
SET app_data = jsonb_set(
    app_data,
    '{model}',
    ('openai:' || (app_data->>'model'))::jsonb
)
WHERE app_data ? 'model'
AND (app_data->>'model') NOT LIKE '%:%';

-- Add comment for documentation
COMMENT ON COLUMN file_versions.app_data IS 'JSON metadata including model field in "provider:model" format (e.g., "openai:gpt-4o")';
