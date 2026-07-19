use crate::progress;
use crate::scan::FileEntry;
use jwalk::WalkDir;
use std::path::Path;
use tauri::AppHandle;

const EMIT_EVERY: u64 = 1000;

/// Find files >= min_size bytes under root, sorted largest first, capped at limit.
/// Emits a progress event every EMIT_EVERY files walked.
pub fn scan_large_files(app: &AppHandle, root: &Path, min_size: u64, limit: usize) -> Vec<FileEntry> {
    let mut walked: u64 = 0;
    let mut files: Vec<FileEntry> = WalkDir::new(root)
        .skip_hidden(false)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            walked += 1;
            if walked % EMIT_EVERY == 0 {
                progress::emit(
                    app,
                    "large",
                    "Đang duyệt cây thư mục",
                    &entry.parent_path().display().to_string(),
                    walked,
                    0,
                );
            }
            let meta = entry.metadata().ok()?;
            let size = meta.len();
            if size < min_size {
                return None;
            }
            Some(FileEntry {
                path: entry.path().display().to_string(),
                size,
                is_dir: false,
            })
        })
        .collect();

    progress::emit(app, "large", "Đang sắp xếp kết quả", "", walked, 0);
    files.sort_by(|a, b| b.size.cmp(&a.size));
    files.truncate(limit);
    files
}
