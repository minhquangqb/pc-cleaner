use crate::progress;
use crate::scan::path_size;
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
pub struct DevArtifact {
    /// The artifact directory itself (node_modules, target, .venv).
    pub path: String,
    /// The project directory that owns the artifact.
    pub project: String,
    /// "node_modules" | "target" | "venv"
    pub kind: String,
    pub size: u64,
    /// Days since the artifact was last modified (last install/build).
    pub age_days: u64,
}

/// Directories that never contain user projects — skipped while walking.
const SKIP_DIRS: &[&str] = &[
    "Library",
    "Applications",
    "Movies",
    "Music",
    "Pictures",
    "Public",
];

/// Deeper than this we stop looking for projects (keeps the walk fast).
const MAX_DEPTH: usize = 8;

struct Found {
    path: PathBuf,
    project: PathBuf,
    kind: &'static str,
}

/// Recursively look for build artifacts. An artifact only counts when its
/// project marker file sits next to it (package.json / Cargo.toml /
/// pyvenv.cfg) so we never flag directories that merely share the name.
fn walk(dir: &Path, depth: usize, app: &AppHandle, visited: &AtomicU64) -> Vec<Found> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let entries: Vec<_> = read.flatten().collect();
    let has_file = |name: &str| entries.iter().any(|e| e.file_name() == *name);
    let has_package_json = has_file("package.json");
    let has_cargo_toml = has_file("Cargo.toml");

    let mut found: Vec<Found> = Vec::new();
    let mut subdirs: Vec<PathBuf> = Vec::new();

    for entry in &entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        // Symlinks are not followed: file_type() reports the link itself.
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();

        if name == "node_modules" {
            if has_package_json {
                found.push(Found {
                    path,
                    project: dir.to_path_buf(),
                    kind: "node_modules",
                });
            }
            continue; // never descend into node_modules
        }
        if name == "target" {
            if has_cargo_toml {
                found.push(Found {
                    path,
                    project: dir.to_path_buf(),
                    kind: "target",
                });
            }
            continue; // build output — nothing to find inside
        }
        if (name == ".venv" || name == "venv") && path.join("pyvenv.cfg").is_file() {
            found.push(Found {
                path,
                project: dir.to_path_buf(),
                kind: "venv",
            });
            continue;
        }
        if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        if depth + 1 <= MAX_DEPTH {
            subdirs.push(path);
        }
    }

    let count = visited.fetch_add(1, Ordering::Relaxed);
    if count % 500 == 0 {
        progress::emit(
            app,
            "dev",
            "finding_projects",
            &dir.display().to_string(),
            count,
            0,
        );
    }

    found.extend(
        subdirs
            .par_iter()
            .flat_map(|d| walk(d, depth + 1, app, visited))
            .collect::<Vec<_>>(),
    );
    found
}

/// Scan the home directory for per-project build artifacts (node_modules,
/// Rust target, Python virtualenvs). All of them are regenerable via a
/// reinstall/rebuild, which is what qualifies them as junk.
pub fn scan_dev_junk(app: &AppHandle) -> Vec<DevArtifact> {
    let home = dirs::home_dir().unwrap_or_default();
    let visited = AtomicU64::new(0);
    let found = walk(&home, 0, app, &visited);

    let total = found.len() as u64;
    let done = AtomicU64::new(0);
    let now = SystemTime::now();

    let mut artifacts: Vec<DevArtifact> = found
        .par_iter()
        .map(|f| {
            progress::emit(
                app,
                "dev",
                "sizing_artifacts",
                &f.path.display().to_string(),
                done.load(Ordering::Relaxed),
                total,
            );
            let size = path_size(&f.path);
            let age_days = f
                .path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| now.duration_since(t).ok())
                .map(|d| d.as_secs() / 86_400)
                .unwrap_or(0);
            done.fetch_add(1, Ordering::Relaxed);
            DevArtifact {
                path: f.path.display().to_string(),
                project: f.project.display().to_string(),
                kind: f.kind.to_string(),
                size,
                age_days,
            }
        })
        .filter(|a| a.size > 0)
        .collect();

    artifacts.sort_by(|a, b| b.size.cmp(&a.size));
    artifacts
}
