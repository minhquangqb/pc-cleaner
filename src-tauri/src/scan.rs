use jwalk::WalkDir;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
}

/// Actual space a file occupies on disk (like `du`), not its logical length.
/// Cloud placeholders (iCloud, Google Drive...) report a full logical size via
/// `len()` while occupying zero blocks locally; sparse/compressed files also
/// differ. On non-Unix platforms this falls back to the logical size.
#[cfg(unix)]
pub fn on_disk_size(meta: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.blocks() * 512
}

#[cfg(not(unix))]
pub fn on_disk_size(meta: &std::fs::Metadata) -> u64 {
    meta.len()
}

/// Total size in bytes of a file or directory tree (parallel walk).
pub fn path_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    WalkDir::new(path)
        .skip_hidden(false)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok().map(|m| m.len()))
        .sum()
}
