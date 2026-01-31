-- 20260131122039_plan_mode_metadata.down.sql
-- Rollback Plan Mode & Build Mode metadata changes

-- Remove the file type check constraint
ALTER TABLE files DROP CONSTRAINT IF EXISTS chk_file_type_valid;

-- Remove the column comment
COMMENT ON COLUMN files.file_type IS 'e.g., ''folder'', ''document'', ''canvas'', ''chat''';
