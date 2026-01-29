-- 20260129114511_archive_cleanup_queue.up.sql

-- Queue for hashes that might be orphans after a version is deleted
CREATE TABLE file_archive_cleanup_queue (
    hash TEXT PRIMARY KEY,
    workspace_id UUID NOT NULL,
    marked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Trigger function to queue cleanup when a version is removed
CREATE OR REPLACE FUNCTION queue_archive_cleanup() RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO file_archive_cleanup_queue (hash, workspace_id)
    VALUES (OLD.hash, OLD.workspace_id)
    ON CONFLICT DO NOTHING;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

-- Attach trigger to file_versions
CREATE TRIGGER trigger_archive_cleanup
AFTER DELETE ON file_versions
FOR EACH ROW
EXECUTE FUNCTION queue_archive_cleanup();
