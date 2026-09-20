use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::db::{self, FileRow};
use crate::error::Result;
use crate::paths::to_rel_slash;

const SIDECAR_DIR: &str = ".meshdrawer";
const SIDECAR_NAME: &str = "metadata.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sidecar {
    version: u32,
    written_at: i64,
    files: Vec<SidecarFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SidecarFile {
    rel_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
    #[serde(default)]
    size: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    #[serde(default)]
    status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    ignored_dupe: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub fn sidecar_path(root: &Path) -> PathBuf {
    root.join(SIDECAR_DIR).join(SIDECAR_NAME)
}

pub fn write_root_sidecar(conn: &rusqlite::Connection, root_id: i64) -> Result<()> {
    let Some(root) = db::get_root(conn, root_id)? else {
        return Ok(());
    };
    let files = db::list_files_for_root(conn, root_id)?;
    let sidecar = Sidecar {
        version: 1,
        written_at: db::now_ms(),
        files: files.into_iter().filter(worth_saving).map(to_sidecar_file).collect(),
    };
    let dest = sidecar_path(Path::new(&root.path));
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(&sidecar)?;
    let tmp = dest.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    if dest.exists() {
        let _ = fs::remove_file(&dest);
    }
    fs::rename(&tmp, &dest)?;
    Ok(())
}

pub fn apply_root_sidecar(conn: &rusqlite::Connection, root_id: i64) -> Result<u32> {
    let Some(root) = db::get_root(conn, root_id)? else {
        return Ok(0);
    };
    let path = sidecar_path(Path::new(&root.path));
    if !path.exists() {
        return Ok(0);
    }
    let raw = fs::read_to_string(&path)?;
    let sidecar: Sidecar = serde_json::from_str(&raw)?;
    let mut applied = 0u32;
    for entry in sidecar.files {
        if apply_entry(conn, root_id, &entry)? {
            applied += 1;
        }
    }
    Ok(applied)
}

pub fn write_all_sidecars(conn: &rusqlite::Connection) -> Result<()> {
    for root in db::list_roots(conn)? {
        write_root_sidecar(conn, root.id)?;
    }
    Ok(())
}

fn worth_saving(file: &FileRow) -> bool {
    file.ignored_dupe
        || (file.status != "unset" && !file.status.is_empty())
        || file
            .notes
            .as_ref()
            .map(|notes| !notes.trim().is_empty())
            .unwrap_or(false)
        || !file.tags.is_empty()
}

fn to_sidecar_file(file: FileRow) -> SidecarFile {
    SidecarFile {
        rel_path: normalize_rel(&file.rel_path),
        sha256: file.sha256.filter(|s| !s.is_empty()),
        size: file.size,
        notes: file
            .notes
            .and_then(|n| if n.trim().is_empty() { None } else { Some(n) }),
        status: if file.status.is_empty() {
            "unset".into()
        } else {
            file.status
        },
        tags: file.tags,
        ignored_dupe: file.ignored_dupe,
    }
}

fn apply_entry(conn: &rusqlite::Connection, root_id: i64, entry: &SidecarFile) -> Result<bool> {
    let rel = normalize_rel(&entry.rel_path);
    let id = match db::file_id_by_rel(conn, root_id, &rel)? {
        Some(id) => id,
        None => match entry.sha256.as_deref() {
            Some(sha) => match db::file_id_by_sha256(conn, root_id, sha)? {
                Some(id) => id,
                None => return Ok(false),
            },
            None => return Ok(false),
        },
    };
    let Some(current) = db::get_file(conn, id)? else {
        return Ok(false);
    };
    let mut changed = false;
    if notes_empty(&current.notes) {
        if let Some(notes) = entry.notes.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            db::set_file_notes(conn, id, notes)?;
            changed = true;
        }
    }
    if current.status == "unset" && !entry.status.is_empty() && entry.status != "unset" {
        db::set_file_status(conn, id, &entry.status)?;
        changed = true;
    }
    for tag in &entry.tags {
        if !current.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            db::add_file_tag(conn, id, tag)?;
            changed = true;
        }
    }
    if entry.ignored_dupe && !current.ignored_dupe {
        db::set_ignored_dupe(conn, id, true)?;
        changed = true;
    }
    Ok(changed)
}

fn notes_empty(notes: &Option<String>) -> bool {
    notes
        .as_ref()
        .map(|s| s.trim().is_empty())
        .unwrap_or(true)
}

fn normalize_rel(rel: &str) -> String {
    to_rel_slash(Path::new(rel)).trim_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        add_file_tag, add_root, get_file, open, set_file_notes, set_file_status, set_sha256,
        upsert_file, FileRecord,
    };
    use tempfile::tempdir;

    fn seed(conn: &rusqlite::Connection, dir: &Path) -> (i64, i64) {
        let (root_id, _) = add_root(conn, dir).unwrap();
        let rec = FileRecord {
            root_id,
            rel_path: "models/dragon.stl".into(),
            abs_path: dir.join("models/dragon.stl").to_string_lossy().into_owned(),
            ext: "stl".into(),
            size: 12,
            mtime_ms: 1,
            companions: vec![],
        };
        let id = upsert_file(conn, &rec).unwrap();
        (root_id, id)
    }

    #[test]
    fn sidecar_roundtrip_restores_tags_notes_status() {
        let dir = tempdir().unwrap();
        let conn = open(&dir.path().join("library.db")).unwrap();
        let (root_id, id) = seed(&conn, dir.path());
        set_file_notes(&conn, id, "print hot").unwrap();
        set_file_status(&conn, id, "favorite").unwrap();
        add_file_tag(&conn, id, "dragon").unwrap();
        write_root_sidecar(&conn, root_id).unwrap();
        assert!(sidecar_path(dir.path()).exists());

        let conn2 = open(&dir.path().join("fresh.db")).unwrap();
        let (root2, id2) = seed(&conn2, dir.path());
        let n = apply_root_sidecar(&conn2, root2).unwrap();
        assert!(n >= 1);
        let file = get_file(&conn2, id2).unwrap().unwrap();
        assert_eq!(file.notes.as_deref(), Some("print hot"));
        assert_eq!(file.status, "favorite");
        assert_eq!(file.tags, vec!["dragon"]);
    }

    #[test]
    fn sidecar_does_not_clobber_existing_notes() {
        let dir = tempdir().unwrap();
        let conn = open(&dir.path().join("library.db")).unwrap();
        let (root_id, id) = seed(&conn, dir.path());
        set_file_notes(&conn, id, "from sidecar").unwrap();
        write_root_sidecar(&conn, root_id).unwrap();

        let conn2 = open(&dir.path().join("fresh.db")).unwrap();
        let (root2, id2) = seed(&conn2, dir.path());
        set_file_notes(&conn2, id2, "keep me").unwrap();
        apply_root_sidecar(&conn2, root2).unwrap();
        let file = get_file(&conn2, id2).unwrap().unwrap();
        assert_eq!(file.notes.as_deref(), Some("keep me"));
    }

    #[test]
    fn sidecar_matches_moved_file_by_hash() {
        let dir = tempdir().unwrap();
        let conn = open(&dir.path().join("library.db")).unwrap();
        let (root_id, id) = seed(&conn, dir.path());
        set_sha256(&conn, id, "abc123").unwrap();
        add_file_tag(&conn, id, "gift").unwrap();
        write_root_sidecar(&conn, root_id).unwrap();

        let conn2 = open(&dir.path().join("fresh.db")).unwrap();
        let (root2, _) = add_root(&conn2, dir.path()).unwrap();
        let rec = FileRecord {
            root_id: root2,
            rel_path: "archive/dragon.stl".into(),
            abs_path: dir.path().join("archive/dragon.stl").to_string_lossy().into_owned(),
            ext: "stl".into(),
            size: 12,
            mtime_ms: 1,
            companions: vec![],
        };
        let id2 = upsert_file(&conn2, &rec).unwrap();
        set_sha256(&conn2, id2, "abc123").unwrap();
        apply_root_sidecar(&conn2, root2).unwrap();
        let file = get_file(&conn2, id2).unwrap().unwrap();
        assert_eq!(file.tags, vec!["gift"]);
    }
}
