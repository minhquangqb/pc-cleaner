mod disk;
mod dupes;
mod junk;
mod large;
mod progress;
mod safety;
mod scan;

use serde::Serialize;
use std::path::PathBuf;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_disk_info,
            scan_junk,
            scan_large_files,
            scan_duplicates,
            clean_paths,
            get_home_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
