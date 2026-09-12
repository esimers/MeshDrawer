CREATE TABLE IF NOT EXISTS scan_errors (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL,
  message TEXT NOT NULL,
  seen_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_files_status ON files(status);
CREATE INDEX IF NOT EXISTS idx_files_root ON files(root_id);
CREATE INDEX IF NOT EXISTS idx_files_thumb_status ON files(thumb_status);
CREATE INDEX IF NOT EXISTS idx_file_tags_tag ON file_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_scan_errors_seen ON scan_errors(seen_at);
