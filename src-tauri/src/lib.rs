mod db;
mod error;
mod ignore;
mod paths;
mod scan;
mod state;
mod watch;

use std::path::Path;
use std::sync::atomic::Ordering;
use std::thread;

use db::{AppInfo, Db, FileRow, LibraryRoot};
use scan::ScanProgress;
use state::ScanState;
use tauri::{AppHandle, Emitter, Manager, State};
use watch::WatchHub;

fn lock_err() -> String {
    "database lock poisoned".to_string()
}

#[tauri::command]
fn get_app_info() -> Result<AppInfo, String> {
    let (data, db_path, thumbs) = paths::ensure_app_dirs().map_err(String::from)?;
    Ok(AppInfo {
        db_path: db_path.to_string_lossy().into_owned(),
        thumbs_dir: thumbs.to_string_lossy().into_owned(),
        data_dir: data.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
fn list_roots(db: State<Db>) -> Result<Vec<LibraryRoot>, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::list_roots(&conn).map_err(String::from)
}

#[tauri::command]
fn list_files(db: State<Db>) -> Result<Vec<FileRow>, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::list_files(&conn).map_err(String::from)
}

#[tauri::command]
fn get_scan_status(scan: State<ScanState>) -> ScanProgress {
    scan.snapshot()
}

#[tauri::command]
fn pause_scan(scan: State<ScanState>) {
    if scan.running.load(Ordering::SeqCst) {
        scan.pause.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
fn resume_scan(scan: State<ScanState>) {
    scan.pause.store(false, Ordering::SeqCst);
}

#[tauri::command]
fn add_library_folder(
    app: AppHandle,
    db: State<Db>,
    watchers: State<WatchHub>,
    path: String,
) -> Result<Vec<LibraryRoot>, String> {
    let (root_id, roots) = {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        db::add_root(&conn, Path::new(&path)).map_err(String::from)?
    };
    watchers.watch_path(Path::new(&path));
    spawn_scan(app, Some(vec![root_id]));
    Ok(roots)
}

#[tauri::command]
fn rescan(app: AppHandle, scan: State<ScanState>) -> Result<(), String> {
    if scan.running.load(Ordering::SeqCst) {
        return Err("A scan is already running".into());
    }
    spawn_scan(app, None);
    Ok(())
}

fn spawn_scan(app: AppHandle, root_ids: Option<Vec<i64>>) {
    let scan = app.state::<ScanState>();
    if !scan.begin() {
        return;
    }

    thread::spawn(move || {
        let db = app.state::<Db>();
        let scan = app.state::<ScanState>();
        let roots = {
            let Ok(conn) = db.0.lock() else {
                scan.finish();
                return;
            };
            match &root_ids {
                Some(ids) => ids
                    .iter()
                    .filter_map(|id| db::get_root(&conn, *id).ok().flatten())
                    .collect::<Vec<_>>(),
                None => db::list_roots(&conn).unwrap_or_default(),
            }
        };

        let mut last = ScanProgress::default();
        for root in roots {
            if scan.cancel.load(Ordering::SeqCst) {
                break;
            }
            match scan::scan_root(
                &db,
                root.id,
                Path::new(&root.path),
                &scan,
                |progress| {
                    last = progress.clone();
                    let _ = app.emit("scan-progress", progress);
                },
            ) {
                Ok(done) => last = done,
                Err(err) => {
                    last.running = false;
                    last.current_path = err.to_string();
                    let _ = app.emit("scan-progress", last.clone());
                }
            }
        }

        last.running = false;
        last.paused = false;
        scan.set_progress(last.clone());
        scan.finish();
        let _ = app.emit("scan-progress", last);
        let _ = app.emit("scan-complete", ());
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let (_data_dir, db_path, _thumbs) = paths::ensure_app_dirs()?;
            let conn = db::open(&db_path)?;
            let empty_index = db::file_count(&conn).unwrap_or(0) == 0;
            let has_roots = !db::list_roots(&conn).unwrap_or_default().is_empty();
            app.manage(Db(std::sync::Mutex::new(conn)));
            app.manage(ScanState::new());
            app.manage(WatchHub::new());

            let handle = app.handle().clone();
            let watchers = handle.state::<WatchHub>();
            watchers.watch_roots(&handle);

            if empty_index && has_roots {
                spawn_scan(handle, None);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            list_roots,
            list_files,
            get_scan_status,
            pause_scan,
            resume_scan,
            add_library_folder,
            rescan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
