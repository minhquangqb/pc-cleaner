mod apps;
mod devjunk;
mod disk;
mod dupes;
mod i18n;
mod junk;
mod large;
mod progress;
mod safety;
mod scan;
mod tray;
mod tree;

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct CleanResult {
    pub freed: u64,
    pub errors: Vec<String>,
}

#[tauri::command]
async fn get_disk_info() -> Result<Vec<disk::DiskInfo>, String> {
    tauri::async_runtime::spawn_blocking(disk::disk_info)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_junk(app: tauri::AppHandle) -> Result<Vec<junk::JunkCategory>, String> {
    tauri::async_runtime::spawn_blocking(move || junk::scan_junk(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_dev_junk(app: tauri::AppHandle) -> Result<Vec<devjunk::DevArtifact>, String> {
    tauri::async_runtime::spawn_blocking(move || devjunk::scan_dev_junk(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_apps(app: tauri::AppHandle) -> Result<Vec<apps::AppInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || apps::list_apps(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn find_app_leftovers(
    bundle_id: String,
    app_name: String,
) -> Result<Vec<scan::FileEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || apps::find_leftovers(&bundle_id, &app_name))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_large_files(
    app: tauri::AppHandle,
    root: String,
    min_size_mb: u64,
    limit: usize,
) -> Result<Vec<scan::FileEntry>, String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()));
    }
    tauri::async_runtime::spawn_blocking(move || {
        large::scan_large_files(&app, &root, min_size_mb * 1024 * 1024, limit.clamp(1, 500))
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_duplicates(
    app: tauri::AppHandle,
    root: String,
    min_size_kb: u64,
) -> Result<Vec<dupes::DupGroup>, String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()));
    }
    tauri::async_runtime::spawn_blocking(move || {
        dupes::scan_duplicates(&app, &root, min_size_kb.max(1) * 1024)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn start_tree_scan(
    app: tauri::AppHandle,
    state: tauri::State<'_, tree::TreeState>,
    root: String,
) -> Result<(), String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()));
    }
    tree::start_scan(app, state.inner().clone(), root);
    Ok(())
}

#[tauri::command]
async fn get_tree_children(
    state: tauri::State<'_, tree::TreeState>,
    path: String,
) -> Result<Vec<scan::FileEntry>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || tree::get_children(&state, Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn set_tree_focus(state: tauri::State<'_, tree::TreeState>, path: String) {
    tree::set_focus(&state, PathBuf::from(path));
}

#[tauri::command]
fn forget_tree_paths(state: tauri::State<'_, tree::TreeState>, items: Vec<(String, u64)>) {
    tree::forget(&state, &items);
}

/// Move the given paths to the system trash (never a permanent delete).
/// Every path is re-validated against the protected list before removal.
#[tauri::command]
async fn clean_paths(paths: Vec<String>) -> Result<CleanResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (freed, errors) = safety::trash_paths(&paths);
        CleanResult { freed, errors }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_home_dir() -> String {
    dirs::home_dir().unwrap_or_default().display().to_string()
}

/// "macos" | "windows" | "linux" — lets the UI hide platform-specific tabs
/// (the uninstaller only works on macOS).
#[tauri::command]
fn get_platform() -> &'static str {
    std::env::consts::OS
}

/// "vi" | "en" — keeps tray menu and notifications in the UI language.
#[tauri::command]
fn set_app_language(app: tauri::AppHandle, lang: String) {
    i18n::set_lang(&lang);
    i18n::persist(&app, i18n::lang());
    tray::update_language(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(tree::TreeState::default())
        .setup(|app| {
            i18n::load(app.handle());
            tray::setup(app)?;
            tray::spawn_watcher(app.handle().clone());
            Ok(())
        })
        .on_window_event(|_window, _event| {
            // macOS/Windows: closing the window keeps the app alive in the
            // tray so the periodic junk check can run; quit via the tray menu.
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            if let tauri::WindowEvent::CloseRequested { api, .. } = _event {
                let _ = _window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_disk_info,
            scan_junk,
            scan_dev_junk,
            list_apps,
            find_app_leftovers,
            scan_large_files,
            scan_duplicates,
            start_tree_scan,
            get_tree_children,
            set_tree_focus,
            forget_tree_paths,
            clean_paths,
            get_home_dir,
            get_platform,
            set_app_language
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_app_handle, _event| {
        // macOS: clicking the Dock icon re-opens the hidden window.
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = _event {
            tray::show_main_window(_app_handle);
        }
    });
}
