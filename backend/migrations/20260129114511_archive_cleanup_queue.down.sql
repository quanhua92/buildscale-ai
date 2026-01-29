-- 20260129114511_archive_cleanup_queue.down.sql

DROP TRIGGER IF EXISTS trigger_archive_cleanup ON file_versions;
DROP FUNCTION IF EXISTS queue_archive_cleanup();
DROP TABLE IF EXISTS file_archive_cleanup_queue;
