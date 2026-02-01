-- 20260131122039_plan_mode_metadata.up.sql
-- Add 'plan' file type and chat mode metadata for Plan Mode & Build Mode

-- Add check constraint for file_type to ensure data integrity
-- This ensures only valid file types can be stored in the database
ALTER TABLE files
ADD CONSTRAINT chk_file_type_valid
CHECK (file_type IN ('folder', 'document', 'canvas', 'chat', 'whiteboard', 'agent', 'skill', 'plan'));

-- Add comment to document the new 'plan' file type
COMMENT ON COLUMN files.file_type IS 'File type: folder, document, canvas, chat, whiteboard, agent, skill, plan';

-- Ensure chat files can store mode and plan_file in their app_data
-- The app_data JSONB column in file_versions already supports arbitrary JSON
-- No schema changes needed - AgentConfig in Rust handles serialization/deserialization

-- Example app_data structure for chat file versions:
-- {
--   "mode": "plan",           -- or "build"
--   "plan_file": "/plans/project-roadmap.plan",  -- absolute path (only in build mode)
--   "model": "gpt-5-mini",
--   "temperature": 0.7,
--   "previous_response_id": "resp_123"
-- }
