use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
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

fn now_ms() -> i64 {
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

pub fn add_root(conn: &Connection, path: &Path) -> Result<Vec<LibraryRoot>> {
    if !path.exists() {
        return Err(AppError::path("folder does not exist", path));
    }
    if !path.is_dir() {
        return Err(AppError::path("not a directory", path));
    }

    let abs = normalize_abs(path);
    let stored = abs.to_string_lossy().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO roots (path, added_at) VALUES (?1, ?2)",
        params![stored, now_ms()],
    )?;
    list_roots(conn)
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
        let roots = add_root(&conn, dir.path()).unwrap();
        assert_eq!(roots.len(), 1);
        let again = add_root(&conn, dir.path()).unwrap();
        assert_eq!(again.len(), 1);
    }
}
