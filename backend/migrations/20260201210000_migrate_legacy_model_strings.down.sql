-- Revert legacy model string migration
-- This removes the "openai:" prefix from model strings

-- Remove provider prefix from model strings
UPDATE file_versions
SET app_data = jsonb_set(
    app_data,
    '{model}',
    substring((app_data->>'model') from '^openai:(.+)$')::jsonb
)
WHERE app_data ? 'model'
AND (app_data->>'model') LIKE 'openai:%';
