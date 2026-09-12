use std::cell::Cell;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::Serialize;
use walkdir::WalkDir;

use crate::db::{self, CompanionRecord, Db, FileRecord};
use crate::error::Result;
use crate::ignore::{
    companion_kind, extension_lower, is_model_ext, matches_exclude, skip_dir_name, skip_file_name,
    skip_path,
};
use crate::paths::{normalize_abs, to_rel_slash};
use crate::state::ScanState;

const BATCH_SIZE: usize = 50;
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub running: bool,
    pub paused: bool,
    pub seen: u64,
    pub indexed: u64,
    pub errors: u64,
    pub current_path: String,
    pub root_path: String,
}

impl Default for ScanProgress {
    fn default() -> Self {
        Self {
            running: false,
            paused: false,
            seen: 0,
            indexed: 0,
            errors: 0,
            current_path: String::new(),
            root_path: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub abs_path: PathBuf,
    pub rel_path: String,
    pub ext: String,
    pub size: u64,
    pub mtime_ms: i64,
    pub companions: Vec<CompanionRecord>,
}

pub fn mtime_ms(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn discover_file(root: &Path, path: &Path) -> Result<Option<DiscoveredFile>> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(None);
    };
    if skip_file_name(name) {
        return Ok(None);
    }
    let Some(ext) = extension_lower(path) else {
        return Ok(None);
    };
    if !is_model_ext(&ext) {
        return Ok(None);
    }

    let abs = normalize_abs(path);
    let rel = match abs.strip_prefix(root) {
        Ok(rel) => to_rel_slash(rel),
        Err(_) => to_rel_slash(path.file_name().map(Path::new).unwrap_or(path)),
    };
    let meta = fs::metadata(&abs)?;
    if !meta.is_file() {
        return Ok(None);
    }

    Ok(Some(DiscoveredFile {
        companions: find_companions(&abs),
        abs_path: abs,
        rel_path: rel,
        ext,
        size: meta.len(),
        mtime_ms: mtime_ms(&meta),
    }))
}

pub fn find_companions(model: &Path) -> Vec<CompanionRecord> {
    let Some(stem) = model.file_stem().and_then(|s| s.to_str()) else {
        return Vec::new();
    };
    let stem_l = stem.to_ascii_lowercase();
    let Some(dir) = model.parent() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path == model {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if skip_file_name(name) {
            continue;
        }
        let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if file_stem.to_ascii_lowercase() != stem_l {
            continue;
        }
        let Some(ext) = extension_lower(&path) else {
            continue;
        };
        if let Some(kind) = companion_kind(&ext) {
            out.push(CompanionRecord {
                abs_path: normalize_abs(&path).to_string_lossy().into_owned(),
                kind: kind.to_string(),
            });
        }
    }
    out
}

pub fn walk_root(
    root: &Path,
    mut on_file: impl FnMut(DiscoveredFile) -> Result<()>,
    mut on_tick: impl FnMut(u64, u64, &Path),
    should_pause: impl Fn() -> bool,
    cancelled: impl Fn() -> bool,
    extra_excludes: &[String],
) -> Result<(u64, u64, HashSet<String>)> {
    let root = normalize_abs(root);
    let mut seen = 0_u64;
    let mut errors = 0_u64;
    let mut indexed_paths = HashSet::new();

    let walker = WalkDir::new(&root).follow_links(false).into_iter();
    for entry in walker.filter_entry(|e| {
        if e.file_type().is_dir() {
            if let Some(name) = e.file_name().to_str() {
                if skip_dir_name(name) && e.path() != root {
                    return false;
                }
            }
        }
        true
    }) {
        while should_pause() && !cancelled() {
            thread::sleep(Duration::from_millis(80));
        }
        if cancelled() {
            break;
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        if entry.file_type().is_dir() {
            continue;
        }

        seen += 1;
        on_tick(seen, errors, entry.path());

        if skip_path(entry.path(), false) || matches_exclude(entry.path(), extra_excludes) {
            continue;
        }

        match discover_file(&root, entry.path()) {
            Ok(Some(found)) => {
                indexed_paths.insert(found.abs_path.to_string_lossy().into_owned());
                on_file(found)?;
            }
            Ok(None) => {}
            Err(_) => {
                errors += 1;
            }
        }
    }

    Ok((seen, errors, indexed_paths))
}

pub fn upsert_discovered(conn: &rusqlite::Connection, root_id: i64, file: &DiscoveredFile) -> Result<i64> {
    let rec = FileRecord {
        root_id,
        rel_path: file.rel_path.clone(),
        abs_path: file.abs_path.to_string_lossy().into_owned(),
        ext: file.ext.clone(),
        size: file.size as i64,
        mtime_ms: file.mtime_ms,
        companions: file.companions.clone(),
    };
    db::upsert_file(conn, &rec)
}

pub fn scan_root(
    db: &Db,
    root_id: i64,
    root_path: &Path,
    state: &ScanState,
    mut emit: impl FnMut(ScanProgress),
) -> Result<ScanProgress> {
    let extra_excludes = {
        let conn = db.0.lock().ok();
        conn.and_then(|c| db::get_setting(&c, "exclude_patterns").ok().flatten())
            .unwrap_or_default()
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
    };
    let indexed = Cell::new(0_u64);
    let mut last_emit = Instant::now() - PROGRESS_EVERY;
    let mut current = String::new();
    let mut batch: Vec<DiscoveredFile> = Vec::with_capacity(BATCH_SIZE);

    let flush = |batch: &mut Vec<DiscoveredFile>| -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }
        let conn = db
            .0
            .lock()
            .map_err(|_| crate::error::AppError::Message("database lock poisoned".into()))?;
        let tx = conn.unchecked_transaction()?;
        for file in batch.iter() {
            upsert_discovered(&tx, root_id, file)?;
        }
        tx.commit()?;
        indexed.set(indexed.get() + batch.len() as u64);
        batch.clear();
        Ok(())
    };

    let snapshot = |seen: u64, errors: u64, current: &str, paused: bool| ScanProgress {
        running: true,
        paused,
        seen,
        indexed: indexed.get(),
        errors,
        current_path: current.to_string(),
        root_path: root_path.display().to_string(),
    };

    let (seen, errors, indexed_paths) = walk_root(
        root_path,
        |file| {
            batch.push(file);
            if batch.len() >= BATCH_SIZE {
                flush(&mut batch)?;
            }
            Ok(())
        },
        |seen, errors, path| {
            current = path.display().to_string();
            if last_emit.elapsed() >= PROGRESS_EVERY {
                let paused = state.pause.load(Ordering::SeqCst);
                let progress = snapshot(seen, errors, &current, paused);
                state.set_progress(progress.clone());
                emit(progress);
                last_emit = Instant::now();
            }
        },
        || state.pause.load(Ordering::SeqCst),
        || state.cancel.load(Ordering::SeqCst),
        &extra_excludes,
    )?;

    flush(&mut batch)?;

    if !state.cancel.load(Ordering::SeqCst) {
        let conn = db
            .0
            .lock()
            .map_err(|_| crate::error::AppError::Message("database lock poisoned".into()))?;
        db::delete_missing(&conn, root_id, &indexed_paths)?;
    }

    let done = ScanProgress {
        running: false,
        paused: false,
        seen,
        indexed: indexed.get(),
        errors,
        current_path: String::new(),
        root_path: root_path.display().to_string(),
    };
    state.set_progress(done.clone());
    emit(done.clone());
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    fn touch(path: &Path, body: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = File::create(path).unwrap();
        f.write_all(body).unwrap();
    }

    #[test]
    fn walks_nested_mixed_case_and_skips_ignored() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        touch(&root.join("dragon.STL"), b"solid");
        touch(&root.join("dragon.png"), b"png");
        touch(&root.join("skip.tmp"), b"nope");
        touch(&root.join("nested").join("wing.3mf"), b"3mf");
        touch(&root.join(".git").join("hidden.stl"), b"no");
        touch(
            &root.join("node_modules").join("pkg").join("lib.obj"),
            b"no",
        );

        let mut found = Vec::new();
        let (seen, errors, paths) = walk_root(
            root,
            |file| {
                found.push(file);
                Ok(())
            },
            |_, _, _| {},
            || false,
            || false,
            &[],
        )
        .unwrap();

        assert_eq!(errors, 0);
        assert!(seen >= 2);
        let rels: Vec<_> = found.iter().map(|f| f.rel_path.as_str()).collect();
        assert!(rels.contains(&"dragon.STL") || rels.iter().any(|r| r.eq_ignore_ascii_case("dragon.stl")));
        assert!(rels.iter().any(|r| r.replace('\\', "/").ends_with("nested/wing.3mf")));
        assert!(!paths.iter().any(|p| p.contains(".git")));
        assert!(!paths.iter().any(|p| p.contains("node_modules")));

        let dragon = found
            .iter()
            .find(|f| f.ext == "stl")
            .expect("stl indexed");
        assert_eq!(dragon.rel_path.replace('\\', "/"), "dragon.STL");
        assert_eq!(dragon.companions.len(), 1);
        assert_eq!(dragon.companions[0].kind, "image");
    }

    #[test]
    fn discover_skips_non_models() {
        let dir = tempdir().unwrap();
        let pdf = dir.path().join("notes.pdf");
        touch(&pdf, b"%PDF");
        assert!(discover_file(dir.path(), &pdf).unwrap().is_none());
    }
}
