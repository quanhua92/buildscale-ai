-- 20260214092000_memory_file_type.up.sql
-- Add 'memory' file type for Memory Management System

-- Drop existing constraint and add new one with 'memory' included
ALTER TABLE files DROP CONSTRAINT IF EXISTS chk_file_type_valid;
ALTER TABLE files ADD CONSTRAINT chk_file_type_valid
CHECK (file_type IN ('folder', 'document', 'canvas', 'chat', 'whiteboard', 'agent', 'skill', 'plan', 'memory'));

-- Update comment to document the new 'memory' file type
COMMENT ON COLUMN files.file_type IS 'File type: folder, document, canvas, chat, whiteboard, agent, skill, plan, memory';
