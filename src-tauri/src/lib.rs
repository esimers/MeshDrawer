mod db;
mod error;
mod ignore;
mod open;
mod paths;
mod scan;
mod state;
mod threemf;
mod thumbs;
mod watch;

use std::fs;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::thread;

use db::{AppInfo, Db, FileRow, LibraryRoot, PlateRow};
use open::SlicerApp;
use scan::ScanProgress;
use state::ScanState;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager, State};
use thumbs::ThumbReady;
use watch::WatchHub;

const MAX_PREVIEW_BYTES: u64 = 200 * 1024 * 1024;

fn lock_err() -> String {
    "database lock poisoned".to_string()
}

fn file_abs(db: &Db, id: i64) -> Result<String, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::get_file(&conn, id)
        .map_err(String::from)?
        .map(|f| f.abs_path)
        .ok_or_else(|| "file not found".into())
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

#[tauri::command]
fn list_slicers() -> Vec<SlicerApp> {
    open::detected_slicers()
}

#[tauri::command]
fn open_in_slicer(db: State<Db>, id: i64, slicer_id: String) -> Result<(), String> {
    let path = file_abs(&db, id)?;
    open::open_with(&slicer_id, Path::new(&path)).map_err(String::from)
}

#[tauri::command]
fn reveal_in_explorer(db: State<Db>, id: i64) -> Result<(), String> {
    let path = file_abs(&db, id)?;
    open::reveal(Path::new(&path)).map_err(String::from)
}

#[tauri::command]
fn read_model_bytes(db: State<Db>, id: i64) -> Result<Response, String> {
    let path = file_abs(&db, id)?;
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_PREVIEW_BYTES {
        return Err("file too large to preview (200MB cap)".into());
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}

#[tauri::command]
fn save_thumb_png(app: AppHandle, db: State<Db>, id: i64, png: Vec<u8>) -> Result<ThumbReady, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    let ready = thumbs::save_rendered_png(&conn, id, &png).map_err(String::from)?;
    drop(conn);
    let _ = app.emit("thumb-ready", ready.clone());
    Ok(ready)
}

#[tauri::command]
fn list_plates(db: State<Db>, file_id: i64) -> Result<Vec<PlateRow>, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::list_plates(&conn, file_id).map_err(String::from)
}

#[tauri::command]
fn load_plate_mesh(db: State<Db>, file_id: i64, plate_index: i64) -> Result<Response, String> {
    let (abs, object_ids) = {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        let file = db::get_file(&conn, file_id)
            .map_err(String::from)?
            .ok_or_else(|| "file not found".to_string())?;
        let plates = db::list_plates(&conn, file_id).map_err(String::from)?;
        let ids = plates
            .into_iter()
            .find(|p| p.plate_index == plate_index)
            .map(|p| p.object_ids)
            .unwrap_or_default();
        (file.abs_path, ids)
    };
    let parts = threemf::load_plate_meshes(Path::new(&abs), &object_ids).map_err(String::from)?;
    Ok(Response::new(threemf::encode_mesh_blob(&parts)))
}

#[tauri::command]
fn set_thumb_status(
    app: AppHandle,
    db: State<Db>,
    id: i64,
    status: String,
    triangle_count: Option<i64>,
) -> Result<ThumbReady, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::update_thumb(&conn, id, &status, None, None, triangle_count).map_err(String::from)?;
    let plate_count = db::get_file(&conn, id)
        .ok()
        .flatten()
        .map(|f| f.plate_count)
        .unwrap_or(0);
    let ready = ThumbReady {
        id,
        thumb_path: None,
        thumb_status: status,
        plate_count,
    };
    drop(conn);
    let _ = app.emit("thumb-ready", ready.clone());
    Ok(ready)
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
        spawn_thumb_extract(app);
    });
}

fn spawn_thumb_extract(app: AppHandle) {
    thread::spawn(move || {
        let db = app.state::<Db>();
        match thumbs::extract_pending(&db) {
            Ok(ready) => {
                for thumb in ready {
                    let _ = app.emit("thumb-ready", thumb);
                }
            }
            Err(err) => eprintln!("embedded thumb extract failed: {err}"),
        }
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
            } else {
                spawn_thumb_extract(handle);
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
            rescan,
            list_slicers,
            open_in_slicer,
            reveal_in_explorer,
            read_model_bytes,
            save_thumb_png,
            set_thumb_status,
            list_plates,
            load_plate_mesh
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
