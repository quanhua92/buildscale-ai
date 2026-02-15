-- 20260214092000_memory_file_type.down.sql
-- Remove 'memory' file type

-- Revert constraint to exclude 'memory'
ALTER TABLE files DROP CONSTRAINT IF EXISTS chk_file_type_valid;
ALTER TABLE files ADD CONSTRAINT chk_file_type_valid
CHECK (file_type IN ('folder', 'document', 'canvas', 'chat', 'whiteboard', 'agent', 'skill', 'plan'));

-- Revert comment
COMMENT ON COLUMN files.file_type IS 'File type: folder, document, canvas, chat, whiteboard, agent, skill, plan';

-- Note: Any existing memory files will need to be handled manually before downgrading
