-- Remove ai_provider_override column from workspaces table

ALTER TABLE workspaces
DROP COLUMN IF EXISTS ai_provider_override;
