mod db;
mod error;
mod files;
mod hash;
mod ignore;
mod open;
mod paths;
mod scan;
mod state;
mod threemf;
mod tessellate;
mod thumbs;
mod watch;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::thread;

use db::{AppInfo, AppSettings, Db, FileRow, LibraryRoot, PlateRow, ScanErrorRow};
use hash::{DuplicateGroup, HashProgress, HashState};
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
fn list_scan_errors(db: State<Db>) -> Result<Vec<ScanErrorRow>, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::list_scan_errors(&conn).map_err(String::from)
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
fn list_slicers(db: State<Db>) -> Result<Vec<SlicerApp>, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    Ok(slicers_from_db(&conn))
}

fn slicers_from_db(conn: &rusqlite::Connection) -> Vec<SlicerApp> {
    let settings = db::load_app_settings(conn).ok();
    let bambu = settings
        .as_ref()
        .map(|s| s.bambu_path.as_str())
        .filter(|p| !p.is_empty());
    let orca = settings
        .as_ref()
        .map(|s| s.orca_path.as_str())
        .filter(|p| !p.is_empty());
    open::detected_slicers_with(bambu, orca)
}

#[tauri::command]
fn open_in_slicer(db: State<Db>, id: i64, slicer_id: String) -> Result<(), String> {
    let path = file_abs(&db, id)?;
    let slicers = {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        slicers_from_db(&conn)
    };
    if slicer_id == "system" {
        return open::open_with("system", Path::new(&path)).map_err(String::from);
    }
    let exe = slicers
        .into_iter()
        .find(|s| s.id == slicer_id && s.available)
        .and_then(|s| s.path)
        .ok_or_else(|| format!("{slicer_id} was not found"))?;
    std::process::Command::new(exe)
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn reveal_in_explorer(db: State<Db>, id: i64) -> Result<(), String> {
    let path = file_abs(&db, id)?;
    open::reveal(Path::new(&path)).map_err(String::from)
}

#[tauri::command]
fn windows_properties(db: State<Db>, id: i64) -> Result<(), String> {
    let path = file_abs(&db, id)?;
    open::windows_properties(Path::new(&path)).map_err(String::from)
}

#[tauri::command]
fn rename_library_file(
    app: AppHandle,
    db: State<Db>,
    watchers: State<WatchHub>,
    id: i64,
    new_name: String,
) -> Result<FileRow, String> {
    let file = {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        db::get_file(&conn, id)
            .map_err(String::from)?
            .ok_or_else(|| "file not found".to_string())?
    };
    let old = PathBuf::from(&file.abs_path);
    let dest = files::next_path(&old, &new_name).map_err(String::from)?;
    watchers.mute(&old);
    watchers.mute(&dest);
    let new_abs = files::rename_on_disk(&old, &new_name).map_err(String::from)?;
    let rel = {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        let root = db::get_root(&conn, file.root_id)
            .map_err(String::from)?
            .ok_or_else(|| "library folder missing".to_string())?;
        let rel = files::rel_path_for(Path::new(&root.path), &new_abs);
        if let Err(err) = db::update_file_location(&conn, id, &new_abs.to_string_lossy(), &rel) {
            let _ = fs::rename(&new_abs, &old);
            return Err(err.to_string());
        }
        rel
    };
    let _ = rel;
    let _ = app.emit("library-changed", ());
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::get_file(&conn, id)
        .map_err(String::from)?
        .ok_or_else(|| "file not found after rename".to_string())
}

#[tauri::command]
fn get_hash_status(hash: State<HashState>) -> HashProgress {
    hash.snapshot()
}

#[tauri::command]
fn list_duplicate_groups(db: State<Db>) -> Result<Vec<DuplicateGroup>, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::list_duplicate_groups(&conn).map_err(String::from)
}

#[tauri::command]
fn ignore_duplicate_group(db: State<Db>, sha256: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::ignore_hash(&conn, &sha256).map_err(String::from)
}

#[tauri::command]
fn set_file_notes(db: State<Db>, id: i64, notes: String) -> Result<FileRow, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::set_file_notes(&conn, id, &notes).map_err(String::from)?;
    db::get_file(&conn, id)
        .map_err(String::from)?
        .ok_or_else(|| "file not found".into())
}

#[tauri::command]
fn set_file_status(db: State<Db>, id: i64, status: String) -> Result<FileRow, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::set_file_status(&conn, id, &status).map_err(String::from)?;
    db::get_file(&conn, id)
        .map_err(String::from)?
        .ok_or_else(|| "file not found".into())
}

#[tauri::command]
fn list_tags(db: State<Db>) -> Result<Vec<String>, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::list_all_tags(&conn).map_err(String::from)
}

#[tauri::command]
fn add_file_tag(db: State<Db>, id: i64, name: String) -> Result<FileRow, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::add_file_tag(&conn, id, &name).map_err(String::from)?;
    db::get_file(&conn, id)
        .map_err(String::from)?
        .ok_or_else(|| "file not found".into())
}

#[tauri::command]
fn remove_file_tag(db: State<Db>, id: i64, name: String) -> Result<FileRow, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::remove_file_tag(&conn, id, &name).map_err(String::from)?;
    db::get_file(&conn, id)
        .map_err(String::from)?
        .ok_or_else(|| "file not found".into())
}

#[tauri::command]
fn get_app_settings(db: State<Db>) -> Result<AppSettings, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::load_app_settings(&conn).map_err(String::from)
}

#[tauri::command]
fn save_app_settings(db: State<Db>, settings: AppSettings) -> Result<AppSettings, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::save_app_settings(&conn, &settings).map_err(String::from)?;
    db::load_app_settings(&conn).map_err(String::from)
}

#[tauri::command]
fn remove_library_folder(db: State<Db>, id: i64) -> Result<Vec<LibraryRoot>, String> {
    let conn = db.0.lock().map_err(|_| lock_err())?;
    db::delete_root(&conn, id).map_err(String::from)
}

#[tauri::command]
fn quarantine_file(
    app: AppHandle,
    db: State<Db>,
    watchers: State<WatchHub>,
    id: i64,
) -> Result<String, String> {
    let file = {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        db::get_file(&conn, id)
            .map_err(String::from)?
            .ok_or_else(|| "file not found".to_string())?
    };
    let src = PathBuf::from(&file.abs_path);
    if !src.exists() {
        return Err("file is missing on disk".into());
    }
    let dest_dir = {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        quarantine_dir(&conn)?
    };
    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "invalid file name".to_string())?;
    let dest = files::unique_in_dir(&dest_dir, name);
    watchers.mute(&src);
    watchers.mute(&dest);
    if fs::rename(&src, &dest).is_err() {
        fs::copy(&src, &dest).map_err(|e| e.to_string())?;
        fs::remove_file(&src).map_err(|e| e.to_string())?;
    }
    {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        db::delete_file_row(&conn, id).map_err(String::from)?;
    }
    let _ = app.emit("library-changed", ());
    Ok(dest.to_string_lossy().into_owned())
}

fn quarantine_dir(conn: &rusqlite::Connection) -> Result<PathBuf, String> {
    if let Some(custom) = db::get_setting(conn, "quarantine_dir").map_err(String::from)? {
        return Ok(PathBuf::from(custom));
    }
    let roots = db::list_roots(conn).map_err(String::from)?;
    if let Some(root) = roots.first() {
        return Ok(Path::new(&root.path).join(".meshdrewer-quarantine"));
    }
    Ok(paths::app_data_dir()
        .map_err(String::from)?
        .join("quarantine"))
}

#[tauri::command]
fn delete_library_file(
    app: AppHandle,
    db: State<Db>,
    watchers: State<WatchHub>,
    id: i64,
) -> Result<(), String> {
    let path = file_abs(&db, id)?;
    let path_buf = PathBuf::from(&path);
    watchers.mute(&path_buf);
    trash::delete(&path_buf).map_err(|e| e.to_string())?;
    {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        db::delete_file_row(&conn, id).map_err(String::from)?;
    }
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
fn load_preview_mesh(db: State<Db>, id: i64) -> Result<Response, String> {
    let file = {
        let conn = db.0.lock().map_err(|_| lock_err())?;
        db::get_file(&conn, id)
            .map_err(String::from)?
            .ok_or_else(|| "file not found".to_string())?
    };
    if !crate::ignore::is_cad_ext(&file.ext) {
        return Err("not a tessellated CAD format".into());
    }
    let parts = tessellate::tessellate_path(Path::new(&file.abs_path), &file.ext)
        .map_err(String::from)?;
    Ok(Response::new(threemf::encode_mesh_blob(&parts)))
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
    let hash = app.state::<HashState>();
    hash.request_cancel();
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
        spawn_hash(app);
    });
}

fn spawn_hash(app: AppHandle) {
    thread::spawn(move || {
        let db = app.state::<Db>();
        let scan = app.state::<ScanState>();
        let hash = app.state::<HashState>();
        match hash::run_pending(&db, &scan, &hash, |progress| {
            let _ = app.emit("hash-progress", progress);
        }) {
            Ok(_) => {
                let _ = app.emit("hash-complete", ());
            }
            Err(err) => eprintln!("hash queue failed: {err}"),
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
            app.manage(HashState::new());
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
            list_scan_errors,
            pause_scan,
            resume_scan,
            add_library_folder,
            rescan,
            list_slicers,
            open_in_slicer,
            reveal_in_explorer,
            read_model_bytes,
            load_preview_mesh,
            save_thumb_png,
            set_thumb_status,
            list_plates,
            load_plate_mesh,
            windows_properties,
            rename_library_file,
            delete_library_file,
            get_hash_status,
            list_duplicate_groups,
            ignore_duplicate_group,
            quarantine_file,
            set_file_notes,
            set_file_status,
            list_tags,
            add_file_tag,
            remove_file_tag,
            get_app_settings,
            save_app_settings,
            remove_library_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
