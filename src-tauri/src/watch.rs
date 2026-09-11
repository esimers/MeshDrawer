use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use tauri::{AppHandle, Emitter, Manager};

use crate::db::{self, Db};
use crate::ignore::{extension_lower, is_model_ext, skip_dir_name, skip_file_name};
use crate::paths::normalize_abs;
use crate::scan::{discover_file, upsert_discovered};

pub struct WatchHub {
    inner: Mutex<Option<Debouncer<RecommendedWatcher, RecommendedCache>>>,
}

impl WatchHub {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn start(&self, app: AppHandle) {
        let mut slot = match self.inner.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        if slot.is_some() {
            return;
        }
        let app_for_cb = app.clone();
        match new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: DebounceEventResult| {
                handle_events(&app_for_cb, result);
            },
        ) {
            Ok(debouncer) => *slot = Some(debouncer),
            Err(err) => {
                eprintln!("failed to start folder watcher: {err}");
            }
        }
    }

    pub fn watch_path(&self, path: &Path) {
        if let Ok(mut slot) = self.inner.lock() {
            if let Some(debouncer) = slot.as_mut() {
                if let Err(err) = debouncer.watch(path, RecursiveMode::Recursive) {
                    eprintln!("watch {}: {err}", path.display());
                }
            }
        }
    }

    pub fn watch_roots(&self, app: &AppHandle) {
        self.start(app.clone());
        let db = app.state::<Db>();
        let Ok(conn) = db.0.lock() else {
            return;
        };
        if let Ok(roots) = db::list_roots(&conn) {
            drop(conn);
            for root in roots {
                self.watch_path(Path::new(&root.path));
            }
        }
    }
}

fn handle_events(app: &AppHandle, result: DebounceEventResult) {
    let events = match result {
        Ok(events) => events,
        Err(_) => return,
    };
    let db = app.state::<Db>();
    let mut changed = false;

    for event in events {
        for path in event.event.paths {
            if noisy(&path) {
                continue;
            }
            match event.event.kind {
                EventKind::Remove(_) => {
                    if apply_remove(&db, &path) {
                        changed = true;
                    }
                }
                EventKind::Create(_) | EventKind::Modify(_) => {
                    if apply_upsert(&db, &path) {
                        changed = true;
                    }
                }
                _ => {
                    if path.exists() {
                        if apply_upsert(&db, &path) {
                            changed = true;
                        }
                    } else if apply_remove(&db, &path) {
                        changed = true;
                    }
                }
            }
        }
    }

    if changed {
        let _ = app.emit("library-changed", ());
    }
}

fn noisy(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(skip_dir_name)
            .unwrap_or(false)
    }) || path
        .file_name()
        .and_then(|n| n.to_str())
        .map(skip_file_name)
        .unwrap_or(false)
}

fn apply_remove(db: &Db, path: &Path) -> bool {
    let Ok(conn) = db.0.lock() else {
        return false;
    };
    let stored = normalize_abs(path).to_string_lossy().into_owned();
    let _ = db::delete_by_abs_path(&conn, &stored);
    let _ = db::delete_under_prefix(&conn, &stored);
    true
}

fn apply_upsert(db: &Db, path: &Path) -> bool {
    let abs = normalize_abs(path);
    if abs.is_dir() {
        return false;
    }
    let Some(ext) = extension_lower(&abs) else {
        return false;
    };
    if !is_model_ext(&ext) {
        return false;
    }

    let Ok(conn) = db.0.lock() else {
        return false;
    };
    let Ok(Some(root_id)) = db::root_id_for_path(&conn, &abs) else {
        return false;
    };
    let Ok(Some(root)) = db::get_root(&conn, root_id) else {
        return false;
    };
    let Ok(Some(found)) = discover_file(Path::new(&root.path), &abs) else {
        return false;
    };
    upsert_discovered(&conn, root_id, &found).is_ok()
}
