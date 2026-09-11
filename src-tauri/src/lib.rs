mod db;
mod error;
mod paths;

use db::{AppInfo, Db, LibraryRoot};
use tauri::Manager;

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
fn list_roots(db: tauri::State<Db>) -> Result<Vec<LibraryRoot>, String> {
    let conn = db.0.lock().map_err(|_| "database lock poisoned".to_string())?;
    db::list_roots(&conn).map_err(String::from)
}

#[tauri::command]
fn add_library_folder(db: tauri::State<Db>, path: String) -> Result<Vec<LibraryRoot>, String> {
    let conn = db.0.lock().map_err(|_| "database lock poisoned".to_string())?;
    db::add_root(&conn, std::path::Path::new(&path)).map_err(String::from)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let (data_dir, db_path, _thumbs) = paths::ensure_app_dirs()?;
            let _ = data_dir;
            let conn = db::open(&db_path)?;
            app.manage(Db(std::sync::Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            list_roots,
            add_library_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
