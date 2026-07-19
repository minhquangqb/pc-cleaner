use crate::{i18n, junk};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_notification::NotificationExt;

const TRAY_ID: &str = "main-tray";
/// First silent check a while after launch (the user just saw the app —
/// no point notifying immediately).
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(10 * 60);
const CHECK_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);
/// Only notify when reclaimable junk exceeds this (5 GB).
const NOTIFY_THRESHOLD: u64 = 5 * 1024 * 1024 * 1024;

fn build_menu<M: Manager<Wry>>(app: &M) -> tauri::Result<Menu<Wry>> {
    let open = MenuItem::with_id(app, "open", i18n::tr("tray_open"), true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", i18n::tr("tray_check"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", i18n::tr("tray_quit"), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    Menu::with_items(app, &[&open, &check, &separator, &quit])
}

/// Rebuild the tray menu with the current language (menu APIs must run on
/// the main thread; menu events keep working because item ids are stable).
pub fn update_language(app: &AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            if let Ok(menu) = build_menu(&app) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    });
}

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("PC Cleaner");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "check" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move { check_junk(&app, true).await });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|_tray, _event| {
            // Windows convention: double-clicking the tray icon reopens the
            // window (single click already shows the menu).
            #[cfg(target_os = "windows")]
            if let tauri::tray::TrayIconEvent::DoubleClick { .. } = _event {
                show_main_window(_tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Periodically re-measure total junk in the background; notify when it
/// crosses the threshold so the user has a reason to come back.
pub fn spawn_watcher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            check_junk(&app, false).await;
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

async fn check_junk(app: &AppHandle, always_notify: bool) {
    let total = tauri::async_runtime::spawn_blocking(junk::junk_total_size)
        .await
        .unwrap_or(0);

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(i18n::tr_size("tray_tooltip", &format_bytes(total))));
    }

    if always_notify || total >= NOTIFY_THRESHOLD {
        let _ = app
            .notification()
            .builder()
            .title("PC Cleaner")
            .body(i18n::tr_size("notify_body", &format_bytes(total)))
            .show();
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
