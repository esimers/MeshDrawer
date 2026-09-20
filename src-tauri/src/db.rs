use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

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
pub struct ScanErrorRow {
    pub path: String,
    pub message: String,
    pub seen_at: i64,
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
    pub thumb_path: Option<String>,
    pub plate_count: i64,
    pub triangle_count: Option<i64>,
    pub last_error: Option<String>,
    pub sha256: Option<String>,
    pub ignored_dupe: bool,
    pub notes: Option<String>,
    pub status: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompanionRecord {
    pub abs_path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlateRow {
    pub plate_index: i64,
    pub name: Option<String>,
    pub thumb_path: Option<String>,
    pub object_ids: Vec<u32>,
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
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -20000;",
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

    let mut current: i64 = conn
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
        current = 1;
    }
    if current < 2 {
        conn.execute_batch(include_str!("../migrations/002_hardening.sql"))?;
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (2, ?1)",
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

pub fn clear_scan_errors(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM scan_errors", [])?;
    Ok(())
}

pub fn record_scan_error(conn: &Connection, path: &str, message: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO scan_errors (path, message, seen_at) VALUES (?1, ?2, ?3)",
        params![path, message, now_ms()],
    )?;
    Ok(())
}

pub fn list_scan_errors(conn: &Connection) -> Result<Vec<ScanErrorRow>> {
    let mut stmt = conn.prepare(
        "SELECT path, message, seen_at FROM scan_errors ORDER BY seen_at DESC LIMIT 200",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ScanErrorRow {
            path: row.get(0)?,
            message: row.get(1)?,
            seen_at: row.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn set_file_last_error(conn: &Connection, id: i64, message: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE files SET last_error = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, message, now_ms()],
    )?;
    Ok(())
}

pub fn file_count(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
        .map_err(AppError::from)
}

pub fn list_files(conn: &Connection) -> Result<Vec<FileRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, root_id, rel_path, abs_path, ext, size, mtime_ms, thumb_status, thumb_path, plate_count, triangle_count, last_error, sha256, ignored_dupe, notes, status
         FROM files
         ORDER BY rel_path COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([], file_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    attach_tags(conn, &mut out)?;
    Ok(out)
}

pub fn list_files_for_root(conn: &Connection, root_id: i64) -> Result<Vec<FileRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, root_id, rel_path, abs_path, ext, size, mtime_ms, thumb_status, thumb_path, plate_count, triangle_count, last_error, sha256, ignored_dupe, notes, status
         FROM files
         WHERE root_id = ?1
         ORDER BY rel_path COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map(params![root_id], file_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    attach_tags(conn, &mut out)?;
    Ok(out)
}

pub fn file_id_by_rel(conn: &Connection, root_id: i64, rel_path: &str) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT id FROM files WHERE root_id = ?1 AND rel_path = ?2",
        params![root_id, rel_path],
        |row| row.get(0),
    )
    .optional()
    .map_err(AppError::from)
}

pub fn file_id_by_sha256(conn: &Connection, root_id: i64, sha256: &str) -> Result<Option<i64>> {
    if sha256.is_empty() {
        return Ok(None);
    }
    conn.query_row(
        "SELECT id FROM files WHERE root_id = ?1 AND sha256 = ?2 LIMIT 1",
        params![root_id, sha256],
        |row| row.get(0),
    )
    .optional()
    .map_err(AppError::from)
}

pub fn set_ignored_dupe(conn: &Connection, id: i64, ignored: bool) -> Result<()> {
    conn.execute(
        "UPDATE files SET ignored_dupe = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, if ignored { 1 } else { 0 }, now_ms()],
    )?;
    Ok(())
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
            last_error = NULL,
            thumb_status = CASE
                WHEN files.size != excluded.size OR files.mtime_ms != excluded.mtime_ms THEN 'none'
                ELSE files.thumb_status
            END,
            thumb_path = CASE
                WHEN files.size != excluded.size OR files.mtime_ms != excluded.mtime_ms THEN NULL
                ELSE files.thumb_path
            END,
            sha256 = CASE
                WHEN files.size != excluded.size OR files.mtime_ms != excluded.mtime_ms THEN NULL
                ELSE files.sha256
            END",
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

fn file_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileRow> {
    Ok(FileRow {
        id: row.get(0)?,
        root_id: row.get(1)?,
        rel_path: row.get(2)?,
        abs_path: row.get(3)?,
        ext: row.get(4)?,
        size: row.get(5)?,
        mtime_ms: row.get(6)?,
        thumb_status: row.get(7)?,
        thumb_path: row.get(8)?,
        plate_count: row.get(9)?,
        triangle_count: row.get(10)?,
        last_error: row.get(11)?,
        sha256: row.get(12)?,
        ignored_dupe: row.get::<_, i64>(13)? != 0,
        notes: row.get(14)?,
        status: row.get(15)?,
        tags: Vec::new(),
    })
}

pub fn attach_tags(conn: &Connection, files: &mut [FileRow]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    let mut map = std::collections::HashMap::<i64, Vec<String>>::new();
    let mut stmt = conn.prepare(
        "SELECT ft.file_id, t.name FROM file_tags ft JOIN tags t ON t.id = ft.tag_id ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;
    for row in rows {
        let (id, name) = row?;
        map.entry(id).or_default().push(name);
    }
    for file in files {
        file.tags = map.remove(&file.id).unwrap_or_default();
    }
    Ok(())
}

pub fn update_file_location(
    conn: &Connection,
    id: i64,
    abs_path: &str,
    rel_path: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE files SET abs_path = ?2, rel_path = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, abs_path, rel_path, now_ms()],
    )?;
    Ok(())
}

pub fn delete_file_row(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM files WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_file(conn: &Connection, id: i64) -> Result<Option<FileRow>> {
    let mut file = conn
        .query_row(
            "SELECT id, root_id, rel_path, abs_path, ext, size, mtime_ms, thumb_status, thumb_path, plate_count, triangle_count, last_error, sha256, ignored_dupe, notes, status
             FROM files WHERE id = ?1",
            params![id],
            file_from_row,
        )
        .optional()
        .map_err(AppError::from)?;
    if let Some(ref mut row) = file {
        attach_tags(conn, std::slice::from_mut(row))?;
    }
    Ok(file)
}

pub fn update_thumb(
    conn: &Connection,
    file_id: i64,
    status: &str,
    thumb_path: Option<&str>,
    plate_count: Option<i64>,
    triangle_count: Option<i64>,
) -> Result<()> {
    conn.execute(
        "UPDATE files SET
            thumb_status = ?2,
            thumb_path = COALESCE(?3, thumb_path),
            plate_count = COALESCE(?4, plate_count),
            triangle_count = COALESCE(?5, triangle_count),
            updated_at = ?6
         WHERE id = ?1",
        params![
            file_id,
            status,
            thumb_path,
            plate_count,
            triangle_count,
            now_ms()
        ],
    )?;
    Ok(())
}

pub fn replace_plates(conn: &Connection, file_id: i64, plates: &[PlateRow]) -> Result<()> {
    conn.execute("DELETE FROM plates WHERE file_id = ?1", params![file_id])?;
    for plate in plates {
        let ids = serde_json::to_string(&plate.object_ids).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "INSERT INTO plates (file_id, plate_index, name, thumb_path, object_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                file_id,
                plate.plate_index,
                plate.name,
                plate.thumb_path,
                ids
            ],
        )?;
    }
    Ok(())
}

pub fn list_plates(conn: &Connection, file_id: i64) -> Result<Vec<PlateRow>> {
    let mut stmt = conn.prepare(
        "SELECT plate_index, name, thumb_path, object_ids FROM plates
         WHERE file_id = ?1 ORDER BY plate_index ASC",
    )?;
    let rows = stmt.query_map(params![file_id], |row| {
        let ids: String = row.get(3)?;
        let object_ids = serde_json::from_str(&ids).unwrap_or_default();
        Ok(PlateRow {
            plate_index: row.get(0)?,
            name: row.get(1)?,
            thumb_path: row.get(2)?,
            object_ids,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn pending_thumbs(conn: &Connection) -> Result<Vec<FileRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, root_id, rel_path, abs_path, ext, size, mtime_ms, thumb_status, thumb_path, plate_count, triangle_count, last_error, sha256, ignored_dupe, notes, status
         FROM files
         WHERE (
             thumb_status = 'none'
             AND (
               ext = '3mf'
               OR id IN (SELECT file_id FROM companions WHERE kind = 'image')
             )
           )
           OR (
             ext = '3mf'
             AND NOT EXISTS (SELECT 1 FROM plates WHERE plates.file_id = files.id)
           )",
    )?;
    let rows = stmt.query_map([], file_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn pending_hashes(conn: &Connection) -> Result<Vec<FileRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, root_id, rel_path, abs_path, ext, size, mtime_ms, thumb_status, thumb_path, plate_count, triangle_count, last_error, sha256, ignored_dupe, notes, status
         FROM files
         WHERE sha256 IS NULL
         ORDER BY size ASC",
    )?;
    let rows = stmt.query_map([], file_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn set_sha256(conn: &Connection, id: i64, sha256: &str) -> Result<()> {
    conn.execute(
        "UPDATE files SET sha256 = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, sha256, now_ms()],
    )?;
    Ok(())
}

pub fn list_duplicate_groups(conn: &Connection) -> Result<Vec<crate::hash::DuplicateGroup>> {
    let mut stmt = conn.prepare(
        "SELECT sha256 FROM files
         WHERE sha256 IS NOT NULL AND ignored_dupe = 0
         GROUP BY sha256
         HAVING COUNT(*) >= 2
         ORDER BY COUNT(*) DESC, MAX(size) DESC",
    )?;
    let hashes: Vec<String> = {
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        out
    };
    let mut groups = Vec::new();
    for sha in hashes {
        let mut files_stmt = conn.prepare(
            "SELECT id, root_id, rel_path, abs_path, ext, size, mtime_ms, thumb_status, thumb_path, plate_count, triangle_count, last_error, sha256, ignored_dupe, notes, status
             FROM files
             WHERE sha256 = ?1 AND ignored_dupe = 0
             ORDER BY rel_path COLLATE NOCASE ASC",
        )?;
        let rows = files_stmt.query_map(params![sha], file_from_row)?;
        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }
        attach_tags(conn, &mut files)?;
        let size = files.first().map(|f| f.size).unwrap_or(0);
        groups.push(crate::hash::DuplicateGroup {
            sha256: sha,
            size,
            files,
        });
    }
    Ok(groups)
}

pub fn ignore_hash(conn: &Connection, sha256: &str) -> Result<()> {
    conn.execute(
        "UPDATE files SET ignored_dupe = 1, updated_at = ?2 WHERE sha256 = ?1",
        params![sha256, now_ms()],
    )?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .map_err(AppError::from)
}

pub fn set_file_notes(conn: &Connection, id: i64, notes: &str) -> Result<()> {
    conn.execute(
        "UPDATE files SET notes = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, notes, now_ms()],
    )?;
    Ok(())
}

pub fn set_file_status(conn: &Connection, id: i64, status: &str) -> Result<()> {
    let status = match status {
        "unprinted" | "printed" | "favorite" => status,
        _ => "unset",
    };
    conn.execute(
        "UPDATE files SET status = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, status, now_ms()],
    )?;
    Ok(())
}

pub fn list_all_tags(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM tags ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn add_file_tag(conn: &Connection, file_id: i64, name: &str) -> Result<Vec<String>> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Message("tag cannot be empty".into()));
    }
    conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![name])?;
    let tag_id: i64 = conn.query_row("SELECT id FROM tags WHERE name = ?1", params![name], |row| {
        row.get(0)
    })?;
    conn.execute(
        "INSERT OR IGNORE INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
        params![file_id, tag_id],
    )?;
    let mut file = get_file(conn, file_id)?.ok_or_else(|| AppError::Message("file not found".into()))?;
    Ok(std::mem::take(&mut file.tags))
}

pub fn remove_file_tag(conn: &Connection, file_id: i64, name: &str) -> Result<Vec<String>> {
    conn.execute(
        "DELETE FROM file_tags WHERE file_id = ?1 AND tag_id = (SELECT id FROM tags WHERE name = ?2)",
        params![file_id, name],
    )?;
    let mut file = get_file(conn, file_id)?.ok_or_else(|| AppError::Message("file not found".into()))?;
    Ok(std::mem::take(&mut file.tags))
}

pub fn add_files_tag(conn: &Connection, ids: &[i64], name: &str) -> Result<Vec<FileRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let tx = conn.unchecked_transaction()?;
    for id in ids {
        add_file_tag(&tx, *id, name)?;
    }
    tx.commit()?;
    files_by_ids(conn, ids)
}

pub fn remove_files_tag(conn: &Connection, ids: &[i64], name: &str) -> Result<Vec<FileRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let tx = conn.unchecked_transaction()?;
    for id in ids {
        remove_file_tag(&tx, *id, name)?;
    }
    tx.commit()?;
    files_by_ids(conn, ids)
}

fn files_by_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<FileRow>> {
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(file) = get_file(conn, *id)? {
            out.push(file);
        }
    }
    Ok(out)
}

pub fn delete_root(conn: &Connection, id: i64) -> Result<Vec<LibraryRoot>> {
    conn.execute("DELETE FROM files WHERE root_id = ?1", params![id])?;
    conn.execute("DELETE FROM roots WHERE id = ?1", params![id])?;
    list_roots(conn)
}

pub fn load_app_settings(conn: &Connection) -> Result<AppSettings> {
    let roots = list_roots(conn)?;
    let default_q = roots
        .first()
        .map(|r| Path::new(&r.path).join(".meshdrewer-quarantine").to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(AppSettings {
        quarantine_dir: get_setting(conn, "quarantine_dir")?.unwrap_or(default_q),
        bambu_path: get_setting(conn, "bambu_path")?.unwrap_or_default(),
        orca_path: get_setting(conn, "orca_path")?.unwrap_or_default(),
        exclude_patterns: get_setting(conn, "exclude_patterns")?.unwrap_or_default(),
    })
}

pub fn save_app_settings(conn: &Connection, settings: &AppSettings) -> Result<()> {
    set_setting(conn, "quarantine_dir", &settings.quarantine_dir)?;
    set_setting(conn, "bambu_path", &settings.bambu_path)?;
    set_setting(conn, "orca_path", &settings.orca_path)?;
    set_setting(conn, "exclude_patterns", &settings.exclude_patterns)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub quarantine_dir: String,
    pub bambu_path: String,
    pub orca_path: String,
    pub exclude_patterns: String,
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn image_companion_path(conn: &Connection, file_id: i64) -> Result<Option<String>> {
    conn.query_row(
        "SELECT abs_path FROM companions WHERE file_id = ?1 AND kind = 'image' LIMIT 1",
        params![file_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(AppError::from)
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
        assert_eq!(version, 2);
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

    #[test]
    fn duplicate_groups_by_hash() {
        let dir = tempdir().unwrap();
        let conn = open(&dir.path().join("library.db")).unwrap();
        let (root_id, _) = add_root(&conn, dir.path()).unwrap();
        let a = FileRecord {
            root_id,
            rel_path: "a.stl".into(),
            abs_path: dir.path().join("a.stl").to_string_lossy().into_owned(),
            ext: "stl".into(),
            size: 5,
            mtime_ms: 1,
            companions: vec![],
        };
        let b = FileRecord {
            root_id,
            rel_path: "copy/a.stl".into(),
            abs_path: dir.path().join("copy-a.stl").to_string_lossy().into_owned(),
            ext: "stl".into(),
            size: 5,
            mtime_ms: 1,
            companions: vec![],
        };
        let id_a = upsert_file(&conn, &a).unwrap();
        let id_b = upsert_file(&conn, &b).unwrap();
        set_sha256(&conn, id_a, "abc").unwrap();
        set_sha256(&conn, id_b, "abc").unwrap();
        let groups = list_duplicate_groups(&conn).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
        ignore_hash(&conn, "abc").unwrap();
        assert!(list_duplicate_groups(&conn).unwrap().is_empty());
    }

    #[test]
    fn tags_notes_and_status() {
        let dir = tempdir().unwrap();
        let conn = open(&dir.path().join("library.db")).unwrap();
        let (root_id, _) = add_root(&conn, dir.path()).unwrap();
        let rec = FileRecord {
            root_id,
            rel_path: "a.stl".into(),
            abs_path: dir.path().join("a.stl").to_string_lossy().into_owned(),
            ext: "stl".into(),
            size: 1,
            mtime_ms: 1,
            companions: vec![],
        };
        let id = upsert_file(&conn, &rec).unwrap();
        set_file_notes(&conn, id, "print this").unwrap();
        set_file_status(&conn, id, "favorite").unwrap();
        let tags = add_file_tag(&conn, id, "dragon").unwrap();
        assert_eq!(tags, vec!["dragon"]);
        let file = get_file(&conn, id).unwrap().unwrap();
        assert_eq!(file.notes.as_deref(), Some("print this"));
        assert_eq!(file.status, "favorite");
        assert_eq!(file.tags, vec!["dragon"]);
    }

    #[test]
    fn tags_apply_to_multiple_files() {
        let dir = tempdir().unwrap();
        let conn = open(&dir.path().join("library.db")).unwrap();
        let (root_id, _) = add_root(&conn, dir.path()).unwrap();
        let id_a = upsert_file(
            &conn,
            &FileRecord {
                root_id,
                rel_path: "a.stl".into(),
                abs_path: dir.path().join("a.stl").to_string_lossy().into_owned(),
                ext: "stl".into(),
                size: 1,
                mtime_ms: 1,
                companions: vec![],
            },
        )
        .unwrap();
        let id_b = upsert_file(
            &conn,
            &FileRecord {
                root_id,
                rel_path: "b.stl".into(),
                abs_path: dir.path().join("b.stl").to_string_lossy().into_owned(),
                ext: "stl".into(),
                size: 1,
                mtime_ms: 1,
                companions: vec![],
            },
        )
        .unwrap();
        add_file_tag(&conn, id_a, "gift").unwrap();
        let tagged = add_files_tag(&conn, &[id_a, id_b], "dragon").unwrap();
        assert_eq!(tagged.len(), 2);
        assert!(tagged.iter().all(|f| f.tags.iter().any(|t| t == "dragon")));
        let after = remove_files_tag(&conn, &[id_a, id_b], "gift").unwrap();
        let a = after.iter().find(|f| f.id == id_a).unwrap();
        assert_eq!(a.tags, vec!["dragon"]);
        let b = after.iter().find(|f| f.id == id_b).unwrap();
        assert_eq!(b.tags, vec!["dragon"]);
    }
}
