use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::error::{AppError, Result};
use crate::paths::normalize_abs;

pub struct Db(pub Mutex<Connection>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRoot {
    pub id: i64,
    pub path: String,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub db_path: String,
    pub thumbs_dir: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRow {
    pub id: i64,
    pub root_id: i64,
    pub rel_path: String,
    pub abs_path: String,
    pub ext: String,
    pub size: i64,
    pub mtime_ms: i64,
    pub thumb_status: String,
    pub plate_count: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompanionRecord {
    pub abs_path: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct FileRecord {
    pub root_id: i64,
    pub rel_path: String,
    pub abs_path: String,
    pub ext: String,
    pub size: i64,
    pub mtime_ms: i64,
    pub companions: Vec<CompanionRecord>,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;",
    )?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );",
    )?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current < 1 {
        conn.execute_batch(include_str!("../migrations/001_init.sql"))?;
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (1, ?1)",
            params![now_ms()],
        )?;
    }

    Ok(())
}

pub fn list_roots(conn: &Connection) -> Result<Vec<LibraryRoot>> {
    let mut stmt = conn.prepare("SELECT id, path, added_at FROM roots ORDER BY added_at ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(LibraryRoot {
            id: row.get(0)?,
            path: row.get(1)?,
            added_at: row.get(2)?,
        })
    })?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn add_root(conn: &Connection, path: &Path) -> Result<(i64, Vec<LibraryRoot>)> {
    if !path.exists() {
        return Err(AppError::path("folder does not exist", path));
    }
    if !path.is_dir() {
        return Err(AppError::path("not a directory", path));
    }

    let abs = normalize_abs(path);
    let stored = abs.to_string_lossy().to_string();
    let id: i64 = conn.query_row(
        "INSERT INTO roots (path, added_at) VALUES (?1, ?2)
         ON CONFLICT(path) DO UPDATE SET path = excluded.path
         RETURNING id",
        params![stored, now_ms()],
        |row| row.get(0),
    )?;
    Ok((id, list_roots(conn)?))
}

pub fn file_count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
        .map_err(AppError::from)
}

pub fn list_files(conn: &Connection) -> Result<Vec<FileRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, root_id, rel_path, abs_path, ext, size, mtime_ms, thumb_status, plate_count, last_error
         FROM files
         ORDER BY rel_path COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(FileRow {
            id: row.get(0)?,
            root_id: row.get(1)?,
            rel_path: row.get(2)?,
            abs_path: row.get(3)?,
            ext: row.get(4)?,
            size: row.get(5)?,
            mtime_ms: row.get(6)?,
            thumb_status: row.get(7)?,
            plate_count: row.get(8)?,
            last_error: row.get(9)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn upsert_file(conn: &Connection, rec: &FileRecord) -> Result<i64> {
    conn.execute(
        "INSERT INTO files (
            root_id, rel_path, abs_path, ext, size, mtime_ms, thumb_status, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'none', ?7)
         ON CONFLICT(abs_path) DO UPDATE SET
            root_id = excluded.root_id,
            rel_path = excluded.rel_path,
            ext = excluded.ext,
            size = excluded.size,
            mtime_ms = excluded.mtime_ms,
            updated_at = excluded.updated_at,
            last_error = NULL",
        params![
            rec.root_id,
            rec.rel_path,
            rec.abs_path,
            rec.ext,
            rec.size,
            rec.mtime_ms,
            now_ms()
        ],
    )?;

    let id: i64 = conn.query_row(
        "SELECT id FROM files WHERE abs_path = ?1",
        params![rec.abs_path],
        |row| row.get(0),
    )?;
    replace_companions(conn, id, &rec.companions)?;
    Ok(id)
}

fn replace_companions(conn: &Connection, file_id: i64, companions: &[CompanionRecord]) -> Result<()> {
    conn.execute("DELETE FROM companions WHERE file_id = ?1", params![file_id])?;
    for companion in companions {
        conn.execute(
            "INSERT INTO companions (file_id, abs_path, kind) VALUES (?1, ?2, ?3)",
            params![file_id, companion.abs_path, companion.kind],
        )?;
    }
    Ok(())
}

pub fn delete_by_abs_path(conn: &Connection, abs_path: &str) -> Result<()> {
    conn.execute("DELETE FROM files WHERE abs_path = ?1", params![abs_path])?;
    Ok(())
}

pub fn delete_under_prefix(conn: &Connection, prefix: &str) -> Result<()> {
    let like = format!("{}%", prefix.replace('\\', "/"));
    conn.execute(
        "DELETE FROM files WHERE replace(abs_path, '\\', '/') LIKE ?1",
        params![like],
    )?;
    Ok(())
}

pub fn delete_missing(conn: &Connection, root_id: i64, seen: &HashSet<String>) -> Result<()> {
    let existing: Vec<String> = {
        let mut stmt = conn.prepare("SELECT abs_path FROM files WHERE root_id = ?1")?;
        let rows = stmt.query_map(params![root_id], |row| row.get(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        out
    };
    for path in existing {
        if !seen.contains(&path) {
            delete_by_abs_path(conn, &path)?;
        }
    }
    Ok(())
}

pub fn root_id_for_path(conn: &Connection, abs_path: &Path) -> Result<Option<i64>> {
    let abs = normalize_abs(abs_path);
    let needle = abs.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
    let roots = list_roots(conn)?;
    let mut best: Option<(usize, i64)> = None;
    for root in roots {
        let candidate = root.path.replace('\\', "/").to_ascii_lowercase();
        if needle == candidate || needle.starts_with(&format!("{candidate}/")) {
            let len = candidate.len();
            if best.map(|(l, _)| len > l).unwrap_or(true) {
                best = Some((len, root.id));
            }
        }
    }
    Ok(best.map(|(_, id)| id))
}

pub fn get_root(conn: &Connection, id: i64) -> Result<Option<LibraryRoot>> {
    conn.query_row(
        "SELECT id, path, added_at FROM roots WHERE id = ?1",
        params![id],
        |row| {
            Ok(LibraryRoot {
                id: row.get(0)?,
                path: row.get(1)?,
                added_at: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrate_is_idempotent() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("library.db");
        let conn = open(&db_path).unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn add_root_roundtrip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("library.db");
        let conn = open(&db_path).unwrap();
        let (id, roots) = add_root(&conn, dir.path()).unwrap();
        assert_eq!(roots.len(), 1);
        assert!(id > 0);
        let (id2, again) = add_root(&conn, dir.path()).unwrap();
        assert_eq!(again.len(), 1);
        assert_eq!(id, id2);
    }

    #[test]
    fn upsert_and_list_files() {
        let dir = tempdir().unwrap();
        let conn = open(&dir.path().join("library.db")).unwrap();
        let (root_id, _) = add_root(&conn, dir.path()).unwrap();
        let rec = FileRecord {
            root_id,
            rel_path: "models/dragon.stl".into(),
            abs_path: dir.path().join("dragon.stl").to_string_lossy().into_owned(),
            ext: "stl".into(),
            size: 12,
            mtime_ms: 1,
            companions: vec![CompanionRecord {
                abs_path: dir.path().join("dragon.png").to_string_lossy().into_owned(),
                kind: "image".into(),
            }],
        };
        let file_id = upsert_file(&conn, &rec).unwrap();
        let again = upsert_file(&conn, &rec).unwrap();
        assert_eq!(file_id, again);
        let files = list_files(&conn).unwrap();
        assert_eq!(files.len(), 1);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM companions WHERE file_id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
