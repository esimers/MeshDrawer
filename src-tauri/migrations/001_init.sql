CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS roots (
  id INTEGER PRIMARY KEY,
  path TEXT UNIQUE NOT NULL,
  added_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS files (
  id INTEGER PRIMARY KEY,
  root_id INTEGER NOT NULL REFERENCES roots(id),
  rel_path TEXT NOT NULL,
  abs_path TEXT NOT NULL UNIQUE,
  ext TEXT NOT NULL,
  size INTEGER NOT NULL,
  mtime_ms INTEGER NOT NULL,
  sha256 TEXT,
  thumb_status TEXT NOT NULL DEFAULT 'none',
  thumb_path TEXT,
  plate_count INTEGER NOT NULL DEFAULT 0,
  triangle_count INTEGER,
  notes TEXT,
  status TEXT NOT NULL DEFAULT 'unset',
  ignored_dupe INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  updated_at INTEGER NOT NULL,
  UNIQUE(root_id, rel_path)
);

CREATE TABLE IF NOT EXISTS plates (
  id INTEGER PRIMARY KEY,
  file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  plate_index INTEGER NOT NULL,
  name TEXT,
  thumb_path TEXT,
  object_ids TEXT,
  UNIQUE(file_id, plate_index)
);

CREATE TABLE IF NOT EXISTS tags (
  id INTEGER PRIMARY KEY,
  name TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS file_tags (
  file_id INTEGER NOT NULL,
  tag_id INTEGER NOT NULL,
  PRIMARY KEY (file_id, tag_id)
);

CREATE TABLE IF NOT EXISTS companions (
  id INTEGER PRIMARY KEY,
  file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  abs_path TEXT NOT NULL,
  kind TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_files_sha256 ON files(sha256);
CREATE INDEX IF NOT EXISTS idx_files_ext ON files(ext);
CREATE INDEX IF NOT EXISTS idx_files_name ON files(rel_path);
CREATE INDEX IF NOT EXISTS idx_files_size ON files(size);
