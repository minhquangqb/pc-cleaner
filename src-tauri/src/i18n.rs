//! Language for the Rust-rendered UI (tray menu, notifications).
//!
//! Scan results and progress events carry stable keys translated by the
//! frontend; only strings rendered natively need translating here. The
//! choice is persisted to a file so the tray comes up in the right
//! language before the webview loads.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set_lang(lang: &str) {
    ENGLISH.store(lang == "en", Ordering::Relaxed);
}

pub fn lang() -> &'static str {
    if ENGLISH.load(Ordering::Relaxed) {
        "en"
    } else {
        "vi"
    }
}

pub fn tr(key: &str) -> &str {
    let en = ENGLISH.load(Ordering::Relaxed);
    match (key, en) {
        ("tray_open", false) => "Mở PC Cleaner",
        ("tray_open", true) => "Open PC Cleaner",
        ("tray_check", false) => "Kiểm tra rác ngay",
        ("tray_check", true) => "Check junk now",
        ("tray_quit", false) => "Thoát PC Cleaner",
        ("tray_quit", true) => "Quit PC Cleaner",
        ("tray_tooltip", false) => "PC Cleaner — rác có thể dọn: {size}",
        ("tray_tooltip", true) => "PC Cleaner — junk to clean: {size}",
        ("notify_body", false) => {
            "Có thể giải phóng {size} dung lượng rác — mở app để dọn dẹp."
        }
        ("notify_body", true) => "{size} of junk can be freed — open the app to clean up.",
        _ => key,
    }
}

/// `tr(key)` with `{size}` filled in.
pub fn tr_size(key: &str, size: &str) -> String {
    tr(key).replace("{size}", size)
}

fn lang_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("lang"))
}

/// Restore the persisted language before the tray is built.
pub fn load(app: &AppHandle) {
    if let Some(path) = lang_file(app) {
        if let Ok(saved) = std::fs::read_to_string(path) {
            set_lang(saved.trim());
        }
    }
}

pub fn persist(app: &AppHandle, lang: &str) {
    if let Some(path) = lang_file(app) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, lang);
    }
}
