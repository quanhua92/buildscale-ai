-- Add indexes for Plan Mode performance optimization
-- These indexes improve query performance for mode-based lookups

-- Index on file_versions.app_data for mode and plan_file queries
-- Note: JSONB indexes require PostgreSQL 12+
-- Creating a GIN index for JSONB app_data to speed up mode/plan_file lookups
CREATE INDEX IF NOT EXISTS idx_file_versions_app_data_gin
ON file_versions USING GIN (app_data);

-- Index on files.file_type for plan file lookups
CREATE INDEX IF NOT EXISTS idx_files_file_type
ON files(file_type);

-- Index on files.path for plan directory queries
CREATE INDEX IF NOT EXISTS idx_files_path
ON files(path);
