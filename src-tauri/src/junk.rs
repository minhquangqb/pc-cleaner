use crate::progress;
use crate::scan::{path_size, FileEntry};
use rayon::prelude::*;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
pub struct JunkCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub total_size: u64,
    pub entries: Vec<FileEntry>,
}

struct CategoryDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    /// If true, list each child of the candidate dirs as its own entry
    /// (so the user can pick per-app caches individually).
    expand_children: bool,
    paths: fn() -> Vec<PathBuf>,
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

/// `%LOCALAPPDATA%` on Windows, `~/Library/Application Support` on macOS,
/// `~/.local/share` on Linux. Only used to build candidate paths — ones that
/// don't exist on the current platform are filtered out later.
fn local_data() -> PathBuf {
    dirs::data_local_dir().unwrap_or_default()
}

/// Cache dirs of every profile in a Chromium-style `User Data` directory
/// (Chrome, Edge, Brave on Windows): `Default`, `Profile 1`, ...
fn chromium_profile_caches(user_data: PathBuf) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(&user_data) else {
        return Vec::new();
    };
    read.flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name == "Default" || name.starts_with("Profile ")
        })
        .flat_map(|e| {
            ["Cache", "Code Cache", "GPUCache"]
                .into_iter()
                .map(move |sub| e.path().join(sub))
        })
        .collect()
}

/// `cache2` dirs of every Firefox profile (Windows keeps cache under
/// `%LOCALAPPDATA%\Mozilla\Firefox\Profiles\<profile>\cache2`).
fn firefox_profile_caches(profiles: PathBuf) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(&profiles) else {
        return Vec::new();
    };
    read.flatten().map(|e| e.path().join("cache2")).collect()
}

fn category_defs() -> Vec<CategoryDef> {
    vec![
        CategoryDef {
            id: "app_caches",
            name: "Cache ứng dụng",
            description: "Cache của các ứng dụng — an toàn để xóa, app sẽ tự tạo lại khi cần.",
            expand_children: true,
            paths: || {
                vec![
                    home().join("Library/Caches"), // macOS
                    home().join(".cache"),         // Linux
                    local_data().join("Microsoft/Windows/INetCache"), // Windows
                ]
            },
        },
        CategoryDef {
            id: "logs",
            name: "Log files",
            description: "File log cũ của ứng dụng và hệ thống (mức người dùng).",
            expand_children: true,
            paths: || {
                vec![
                    home().join("Library/Logs"),       // macOS
                    local_data().join("CrashDumps"),   // Windows
                ]
            },
        },
        CategoryDef {
            id: "browser_caches",
            name: "Cache trình duyệt",
            description: "Cache của Chrome, Firefox, Edge... Không xóa lịch sử hay mật khẩu.",
            expand_children: false,
            paths: || {
                let mut paths = vec![
                    // macOS
                    home().join("Library/Caches/Google/Chrome"),
                    home().join("Library/Caches/com.google.Chrome"),
                    home().join("Library/Caches/Firefox"),
                    home().join("Library/Caches/com.microsoft.edgemac"),
                    home().join("Library/Caches/BraveSoftware"),
                    home().join("Library/Caches/Arc"),
                ];
                // Windows: cache lives inside each browser profile
                paths.extend(chromium_profile_caches(
                    local_data().join("Google/Chrome/User Data"),
                ));
                paths.extend(chromium_profile_caches(
                    local_data().join("Microsoft/Edge/User Data"),
                ));
                paths.extend(chromium_profile_caches(
                    local_data().join("BraveSoftware/Brave-Browser/User Data"),
                ));
                paths.extend(firefox_profile_caches(
                    local_data().join("Mozilla/Firefox/Profiles"),
                ));
                paths
            },
        },
        CategoryDef {
            id: "dev_caches",
            name: "Cache công cụ dev",
            description:
                "npm, pnpm, yarn, Cargo, Homebrew, Gradle, Xcode DerivedData — tái tạo được khi build/cài lại.",
            expand_children: false,
            paths: || {
                vec![
                    // Cross-platform (dotdirs in home)
                    home().join(".npm/_cacache"),
                    home().join(".pnpm-store"),
                    home().join(".cargo/registry/cache"),
                    home().join(".gradle/caches"),
                    // macOS
                    home().join("Library/pnpm/store"),
                    home().join("Library/Caches/Yarn"),
                    home().join("Library/Caches/Homebrew"),
                    home().join("Library/Developer/Xcode/DerivedData"),
                    home().join("Library/Caches/CocoaPods"),
                    home().join("Library/Caches/pip"),
                    home().join("Library/Caches/go-build"),
                    // Windows (%LOCALAPPDATA%)
                    local_data().join("npm-cache"),
                    local_data().join("pnpm/store"),
                    local_data().join("Yarn/Cache"),
                    local_data().join("pip/cache"),
                    local_data().join("go-build"),
                    local_data().join("NuGet/v3-cache"),
                ]
            },
        },
        CategoryDef {
            id: "temp",
            name: "File tạm",
            description: "Thư mục temp của người dùng.",
            expand_children: true,
            paths: || vec![std::env::temp_dir()],
        },
    ]
}

fn candidates_for(def: &CategoryDef) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for base in (def.paths)() {
        if !base.exists() {
            continue;
        }
        if def.expand_children {
            if let Ok(children) = std::fs::read_dir(&base) {
                for child in children.flatten() {
                    candidates.push(child.path());
                }
            }
        } else {
            candidates.push(base);
        }
    }
    candidates
}

/// Total reclaimable junk across all categories, computed without emitting
/// progress events — used by the background tray check.
pub fn junk_total_size() -> u64 {
    category_defs()
        .par_iter()
        .map(|def| {
            candidates_for(def)
                .par_iter()
                .map(|p| path_size(p))
                .sum::<u64>()
        })
        .sum()
}

/// Scan all junk categories. Only paths that actually exist are reported.
/// Emits per-item progress events while sizing directories.
pub fn scan_junk(app: &AppHandle) -> Vec<JunkCategory> {
    // Gather all candidate paths first so we know the total upfront.
    let with_candidates: Vec<(CategoryDef, Vec<PathBuf>)> = category_defs()
        .into_iter()
        .map(|def| {
            let candidates = candidates_for(&def);
            (def, candidates)
        })
        .collect();

    let total: u64 = with_candidates.iter().map(|(_, c)| c.len() as u64).sum();
    let done = AtomicU64::new(0);

    let mut categories: Vec<JunkCategory> = with_candidates
        .into_par_iter()
        .map(|(def, candidates)| {
            let mut entries: Vec<FileEntry> = candidates
                .par_iter()
                .map(|p| {
                    progress::emit(
                        app,
                        "junk",
                        // Stable key — the frontend renders it per locale.
                        &format!("sizing:{}", def.id),
                        &p.display().to_string(),
                        done.load(Ordering::Relaxed),
                        total,
                    );
                    let entry = FileEntry {
                        path: p.display().to_string(),
                        size: path_size(p),
                        is_dir: p.is_dir(),
                    };
                    done.fetch_add(1, Ordering::Relaxed);
                    entry
                })
                .filter(|e| e.size > 0)
                .collect();
            entries.sort_by(|a, b| b.size.cmp(&a.size));

            JunkCategory {
                id: def.id.to_string(),
                name: def.name.to_string(),
                description: def.description.to_string(),
                total_size: entries.iter().map(|e| e.size).sum(),
                entries,
            }
        })
        .filter(|c| !c.entries.is_empty())
        .collect();

    categories.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    categories
}
