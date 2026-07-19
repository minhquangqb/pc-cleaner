use crate::progress;
use jwalk::WalkDir;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
pub struct DupGroup {
    pub hash: String,
    pub size: u64,
    /// Bytes reclaimable by keeping one copy.
    pub wasted: u64,
    pub paths: Vec<String>,
}

const PARTIAL_READ: usize = 64 * 1024;
const WALK_EMIT_EVERY: u64 = 1000;
const HASH_EMIT_EVERY: u64 = 20;

fn partial_hash(path: &Path) -> Option<u64> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; PARTIAL_READ];
    let n = file.read(&mut buf).ok()?;
    let hash = blake3::hash(&buf[..n]);
    let bytes: [u8; 8] = hash.as_bytes()[..8].try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

fn full_hash(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(file).ok()?;
    Some(hasher.finalize().to_hex().to_string())
}

/// Find duplicate files under root: group by size, then partial hash,
/// then confirm with a full BLAKE3 hash. Groups sorted by wasted bytes.
/// Emits progress events for each stage.
pub fn scan_duplicates(app: &AppHandle, root: &Path, min_size: u64) -> Vec<DupGroup> {
    // Stage 1: group by exact size.
    let mut walked: u64 = 0;
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for entry in WalkDir::new(root)
        .skip_hidden(true)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        walked += 1;
        if walked % WALK_EMIT_EVERY == 0 {
            progress::emit(
                app,
                "dupes",
                "Giai đoạn 1/3 — liệt kê file",
                &entry.parent_path().display().to_string(),
                walked,
                0,
            );
        }
        if let Ok(meta) = entry.metadata() {
            let size = meta.len();
            if size >= min_size {
                by_size.entry(size).or_default().push(entry.path());
            }
        }
    }

    // Stage 2: within same-size groups, group by hash of the first 64 KB.
    let size_groups: Vec<(u64, Vec<PathBuf>)> =
        by_size.into_iter().filter(|(_, v)| v.len() > 1).collect();
    let partial_total: u64 = size_groups.iter().map(|(_, v)| v.len() as u64).sum();
    let partial_done = AtomicU64::new(0);

    let candidate_groups: Vec<(u64, Vec<PathBuf>)> = size_groups
        .into_par_iter()
        .flat_map(|(size, paths)| {
            let mut by_partial: HashMap<u64, Vec<PathBuf>> = HashMap::new();
            for path in paths {
                let done = partial_done.fetch_add(1, Ordering::Relaxed) + 1;
                if done % HASH_EMIT_EVERY == 0 || done == partial_total {
                    progress::emit(
                        app,
                        "dupes",
                        "Giai đoạn 2/3 — so sánh 64KB đầu file",
                        &path.display().to_string(),
                        done,
                        partial_total,
                    );
                }
                if let Some(h) = partial_hash(&path) {
                    by_partial.entry(h).or_default().push(path);
                }
            }
            by_partial
                .into_values()
                .filter(|v| v.len() > 1)
                .map(|v| (size, v))
                .collect::<Vec<_>>()
        })
        .collect();

    // Stage 3: confirm with full-content hash.
    let full_total: u64 = candidate_groups.iter().map(|(_, v)| v.len() as u64).sum();
    let full_done = AtomicU64::new(0);

    let mut groups: Vec<DupGroup> = candidate_groups
        .into_par_iter()
        .flat_map(|(size, paths)| {
            let mut by_full: HashMap<String, Vec<PathBuf>> = HashMap::new();
            for path in paths {
                let done = full_done.fetch_add(1, Ordering::Relaxed) + 1;
                progress::emit(
                    app,
                    "dupes",
                    "Giai đoạn 3/3 — xác nhận bằng full hash",
                    &path.display().to_string(),
                    done,
                    full_total,
                );
                if let Some(h) = full_hash(&path) {
                    by_full.entry(h).or_default().push(path);
                }
            }
            by_full
                .into_iter()
                .filter(|(_, v)| v.len() > 1)
                .map(|(hash, v)| DupGroup {
                    hash,
                    size,
                    wasted: size * (v.len() as u64 - 1),
                    paths: v.into_iter().map(|p| p.display().to_string()).collect(),
                })
                .collect::<Vec<_>>()
        })
        .collect();

    groups.sort_by(|a, b| b.wasted.cmp(&a.wasted));
    groups
}
