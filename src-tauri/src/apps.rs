use crate::progress;
use crate::scan::{path_size, FileEntry};
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub path: String,
    pub name: String,
    pub bundle_id: String,
    pub size: u64,
    /// Days since the app was last opened (Spotlight metadata).
    /// None = unknown (never opened, or Spotlight has no record).
    pub last_used_days: Option<u64>,
    /// App icon as a `data:image/png;base64,...` URI (64px), if extractable.
    pub icon: Option<String>,
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

/// Never offer to uninstall ourselves.
const OWN_BUNDLE_ID: &str = "com.dungqb.pc-cleaner";

/// List installed applications (top-level `.app` bundles in /Applications
/// and ~/Applications). Only meaningful on macOS; elsewhere the directories
/// simply don't exist and the list is empty.
pub fn list_apps(app: &AppHandle) -> Vec<AppInfo> {
    let mut bundles: Vec<PathBuf> = Vec::new();
    for root in [PathBuf::from("/Applications"), home().join("Applications")] {
        let Ok(read) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "app") {
                bundles.push(path);
            }
        }
    }

    let total = bundles.len() as u64;
    let done = AtomicU64::new(0);

    let mut apps: Vec<AppInfo> = bundles
        .par_iter()
        .filter_map(|bundle| {
            progress::emit(
                app,
                "apps",
                "Đang đọc thông tin ứng dụng",
                &bundle.display().to_string(),
                done.load(Ordering::Relaxed),
                total,
            );
            let info = read_bundle(bundle);
            done.fetch_add(1, Ordering::Relaxed);
            info
        })
        .filter(|a| a.bundle_id != OWN_BUNDLE_ID)
        .collect();

    apps.sort_by(|a, b| b.size.cmp(&a.size));
    apps
}

fn read_bundle(bundle: &Path) -> Option<AppInfo> {
    let info: plist::Value = plist::from_file(bundle.join("Contents/Info.plist")).ok()?;
    let dict = info.as_dictionary()?;
    let get = |key: &str| dict.get(key).and_then(|v| v.as_string()).map(str::to_string);
    let bundle_id = get("CFBundleIdentifier")?;
    let name = get("CFBundleDisplayName")
        .or_else(|| get("CFBundleName"))
        .unwrap_or_else(|| {
            bundle
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
    let icon = app_icon_data_uri(bundle, &bundle_id, dict);
    Some(AppInfo {
        path: bundle.display().to_string(),
        name,
        bundle_id,
        size: path_size(bundle),
        last_used_days: last_used_days(bundle),
        icon,
    })
}

/// Extract the app icon: locate the bundle's `.icns`, convert it to a 64px
/// PNG with `sips` (cached under the user cache dir so later scans are
/// instant), and return it inline as a data URI.
#[cfg(target_os = "macos")]
fn app_icon_data_uri(bundle: &Path, bundle_id: &str, dict: &plist::Dictionary) -> Option<String> {
    use base64::Engine;

    let resources = bundle.join("Contents/Resources");
    let mut icns = dict
        .get("CFBundleIconFile")
        .and_then(|v| v.as_string())
        .map(|name| {
            let mut p = resources.join(name);
            if p.extension().is_none() {
                p.set_extension("icns");
            }
            p
        })
        .filter(|p| p.is_file());
    if icns.is_none() {
        if let Ok(read) = std::fs::read_dir(&resources) {
            icns = read
                .flatten()
                .map(|e| e.path())
                .find(|p| p.extension().is_some_and(|ext| ext == "icns"));
        }
    }
    let icns = icns?;

    let cache_dir = dirs::cache_dir()?.join("pc-cleaner/icons");
    std::fs::create_dir_all(&cache_dir).ok()?;
    let png = cache_dir.join(format!("{}.png", bundle_id.replace('/', "_")));

    // Re-convert only when the source icon is newer than the cached PNG.
    let stale = match (png.metadata(), icns.metadata()) {
        (Ok(p), Ok(i)) => match (p.modified(), i.modified()) {
            (Ok(pm), Ok(im)) => im > pm,
            _ => false,
        },
        _ => true,
    };
    if stale {
        let ok = std::process::Command::new("sips")
            .args(["-z", "64", "64", "-s", "format", "png"])
            .arg(&icns)
            .arg("--out")
            .arg(&png)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            let _ = std::fs::remove_file(&png);
            return None;
        }
    }
    let bytes = std::fs::read(&png).ok().filter(|b| !b.is_empty())?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[cfg(not(target_os = "macos"))]
fn app_icon_data_uri(_bundle: &Path, _bundle_id: &str, _dict: &plist::Dictionary) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn last_used_days(bundle: &Path) -> Option<u64> {
    let out = std::process::Command::new("mdls")
        .args(["-name", "kMDItemLastUsedDate", "-raw"])
        .arg(bundle)
        .output()
        .ok()?;
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if raw.is_empty() || raw == "(null)" {
        return None;
    }
    let t = chrono::DateTime::parse_from_str(&raw, "%Y-%m-%d %H:%M:%S %z").ok()?;
    let days = (chrono::Utc::now().timestamp() - t.timestamp()).max(0) / 86_400;
    Some(days as u64)
}

#[cfg(not(target_os = "macos"))]
fn last_used_days(_bundle: &Path) -> Option<u64> {
    None
}

/// Children of `dir` whose file name matches `predicate`.
fn matching_children(dir: &Path, predicate: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .filter(|e| predicate(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect()
}

/// Files an app leaves behind in ~/Library, located by bundle id (exact or
/// prefix match) and — for a few conventional dirs — by app name. Everything
/// returned lives under home, so the normal safety validation applies on
/// delete.
pub fn find_leftovers(bundle_id: &str, app_name: &str) -> Vec<FileEntry> {
    let lib = home().join("Library");
    let mut candidates: Vec<PathBuf> = Vec::new();

    for dir in [
        "Caches",
        "Application Support",
        "Logs",
        "Containers",
        "WebKit",
        "HTTPStorages",
        "Application Scripts",
    ] {
        candidates.push(lib.join(dir).join(bundle_id));
    }
    // Name-based matches only for a few conventional dirs, and only when the
    // name is specific enough to avoid false positives.
    if app_name.len() >= 3 {
        for dir in ["Caches", "Application Support", "Logs"] {
            candidates.push(lib.join(dir).join(app_name));
        }
    }
    candidates.push(
        lib.join("Saved Application State")
            .join(format!("{bundle_id}.savedState")),
    );
    candidates.push(lib.join("Cookies").join(format!("{bundle_id}.binarycookies")));

    // Names that carry suffixes (com.foo.bar.plist, com.foo.bar.helper.plist).
    for dir in ["Preferences", "LaunchAgents", "HTTPStorages"] {
        candidates.extend(matching_children(&lib.join(dir), |n| {
            n.starts_with(bundle_id)
        }));
    }
    // Group containers look like "group.com.foo.bar" or "TEAMID.com.foo.bar".
    candidates.extend(matching_children(&lib.join("Group Containers"), |n| {
        n.contains(bundle_id)
    }));

    candidates.sort();
    candidates.dedup();

    let mut entries: Vec<FileEntry> = candidates
        .into_par_iter()
        .filter(|p| p.exists())
        .map(|p| FileEntry {
            path: p.display().to_string(),
            size: path_size(&p),
            is_dir: p.is_dir(),
        })
        .collect();
    entries.sort_by(|a, b| b.size.cmp(&a.size));
    entries
}
