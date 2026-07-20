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
/// differ.
#[cfg(unix)]
pub fn on_disk_size(meta: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.blocks() * 512
}

/// Windows: cloud placeholders (OneDrive Files On-Demand...) carry recall /
/// offline attributes and occupy ~0 bytes locally; count them as 0. Other
/// files fall back to the logical size (NTFS compression is over-reported —
/// acceptable).
#[cfg(windows)]
pub fn on_disk_size(meta: &std::fs::Metadata) -> u64 {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
    const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    let placeholder =
        FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;
    if meta.file_attributes() & placeholder != 0 {
        return 0;
    }
    meta.len()
}

#[cfg(not(any(unix, windows)))]
pub fn on_disk_size(meta: &std::fs::Metadata) -> u64 {
    meta.len()
}

/// Total size in bytes of a file or directory tree.
///
/// The walk is serial on purpose: callers parallelize per candidate with
/// rayon, and jwalk's parallel walk silently drops entries when started from
/// inside a busy rayon worker thread (undercounted sizes, categories
/// vanishing from the junk scan).
pub fn path_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    WalkDir::new(path)
        .skip_hidden(false)
        .follow_links(false)
        .parallelism(jwalk::Parallelism::Serial)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok().map(|m| m.len()))
        .sum()
}
