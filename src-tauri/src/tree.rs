use crate::progress;
use crate::scan::{on_disk_size, FileEntry};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub const TREE_DONE_EVENT: &str = "tree://done";
const EMIT_EVERY: u64 = 5000;
const MAX_WORKERS: usize = 8;

#[derive(Default)]
struct Inner {
    /// Recursive size per directory, growing live while the background walk runs.
    dir_sizes: Mutex<HashMap<PathBuf, u64>>,
    /// Bumped on every new scan; worker threads abort when it changes.
    generation: AtomicU64,
    /// Subtree the user is currently browsing — sized before everything else.
    focus: Mutex<PathBuf>,
    focus_version: AtomicU64,
}

/// Shared sizing index. Listing is always a live `read_dir`; only directory
/// totals come from here, so browsing never waits for the scan.
#[derive(Clone, Default)]
pub struct TreeState(Arc<Inner>);

/// Pending directories, split by whether they fall under the focused subtree.
/// Workers drain `hot` (LIFO → depth-first, so focused folders converge fast)
/// before touching `cold`.
struct Queue {
    hot: Vec<PathBuf>,
    cold: Vec<PathBuf>,
    focus: PathBuf,
    focus_version: u64,
    /// Directories queued or being processed; 0 means the walk is complete.
    outstanding: usize,
}

/// Kick off a background walk that accumulates every file's size into all of
/// its ancestor directories. Returns immediately; emits progress while walking
/// and `tree://done` (with the grand total) when finished. Starting a new scan
/// cancels the previous one.
pub fn start_scan(app: AppHandle, state: TreeState, root: PathBuf) {
    let generation = state.0.generation.fetch_add(1, Ordering::SeqCst) + 1;
    state.0.dir_sizes.lock().unwrap().clear();
    *state.0.focus.lock().unwrap() = root.clone();
    let focus_version = state.0.focus_version.fetch_add(1, Ordering::SeqCst) + 1;

    let queue = Arc::new((
        Mutex::new(Queue {
            hot: vec![root.clone()],
            cold: Vec::new(),
            focus: root.clone(),
            focus_version,
            outstanding: 1,
        }),
        Condvar::new(),
    ));
    let walked = Arc::new(AtomicU64::new(0));

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(MAX_WORKERS);
    for _ in 0..workers {
        let app = app.clone();
        let state = state.clone();
        let root = root.clone();
        let queue = Arc::clone(&queue);
        let walked = Arc::clone(&walked);
        std::thread::spawn(move || worker_loop(app, state, root, queue, walked, generation));
    }
}

/// Point the scanner at the subtree the user just opened; pending directories
/// under it jump to the front of the queue.
pub fn set_focus(state: &TreeState, path: PathBuf) {
    *state.0.focus.lock().unwrap() = path;
    state.0.focus_version.fetch_add(1, Ordering::SeqCst);
}

fn worker_loop(
    app: AppHandle,
    state: TreeState,
    root: PathBuf,
    queue: Arc<(Mutex<Queue>, Condvar)>,
    walked: Arc<AtomicU64>,
    generation: u64,
) {
    let (lock, cvar) = &*queue;
    loop {
        let dir = {
            let mut q = lock.lock().unwrap();
            loop {
                if state.0.generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                let version = state.0.focus_version.load(Ordering::SeqCst);
                if q.focus_version != version {
                    let focus = state.0.focus.lock().unwrap().clone();
                    repartition(&mut q, focus, version);
                }
                if let Some(dir) = q.hot.pop().or_else(|| q.cold.pop()) {
                    break dir;
                }
                if q.outstanding == 0 {
                    return;
                }
                // Timeout so threads of a cancelled scan never sleep forever.
                q = cvar.wait_timeout(q, Duration::from_millis(200)).unwrap().0;
            }
        };

        let mut file_bytes: u64 = 0;
        let mut file_count: u64 = 0;
        let mut subdirs: Vec<PathBuf> = Vec::new();
        if let Ok(read) = fs::read_dir(&dir) {
            for entry in read.filter_map(|entry| entry.ok()) {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                // Symlinks and special entries are ignored to avoid double counting.
                if file_type.is_dir() {
                    subdirs.push(entry.path());
                } else if file_type.is_file() {
                    file_count += 1;
                    file_bytes += entry.metadata().map(|m| on_disk_size(&m)).unwrap_or(0);
                }
            }
        }

        {
            let mut sizes = state.0.dir_sizes.lock().unwrap();
            if state.0.generation.load(Ordering::SeqCst) != generation {
                return; // don't write stale data after a newer scan cleared the map
            }
            *sizes.entry(dir.clone()).or_insert(0) += file_bytes;
            if file_bytes > 0 {
                for ancestor in dir.ancestors().skip(1) {
                    if !ancestor.starts_with(&root) {
                        break;
                    }
                    *sizes.entry(ancestor.to_path_buf()).or_insert(0) += file_bytes;
                }
            }
        }

        let done_count = walked.fetch_add(file_count + 1, Ordering::Relaxed) + file_count + 1;
        if done_count / EMIT_EVERY != (done_count - file_count - 1) / EMIT_EVERY {
            progress::emit(
                &app,
                "tree",
                "sizing",
                &dir.display().to_string(),
                done_count,
                0,
            );
        }

        let mut q = lock.lock().unwrap();
        let pushed = !subdirs.is_empty();
        q.outstanding += subdirs.len();
        for sub in subdirs {
            if sub.starts_with(&q.focus) {
                q.hot.push(sub);
            } else {
                q.cold.push(sub);
            }
        }
        q.outstanding -= 1;
        let finished = q.outstanding == 0;
        drop(q);
        if pushed || finished {
            cvar.notify_all();
        }
        if finished {
            if state.0.generation.load(Ordering::SeqCst) == generation {
                let total = state
                    .0
                    .dir_sizes
                    .lock()
                    .unwrap()
                    .get(&root)
                    .copied()
                    .unwrap_or(0);
                progress::emit(&app, "tree", "done", "", done_count, done_count.max(1));
                let _ = app.emit(TREE_DONE_EVENT, total);
            }
            return;
        }
    }
}

fn repartition(q: &mut Queue, focus: PathBuf, version: u64) {
    let pending: Vec<PathBuf> = q.hot.drain(..).chain(q.cold.drain(..)).collect();
    for path in pending {
        if path.starts_with(&focus) {
            q.hot.push(path);
        } else {
            q.cold.push(path);
        }
    }
    q.focus = focus;
    q.focus_version = version;
}

/// Live listing of a directory: names via `read_dir`, file sizes via metadata,
/// directory sizes from the (possibly still growing) index. Largest first.
pub fn get_children(state: &TreeState, dir: &Path) -> Result<Vec<FileEntry>, String> {
    let read = fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {e}"))?;
    let mut out: Vec<FileEntry> = Vec::new();
    {
        let sizes = state.0.dir_sizes.lock().unwrap();
        for entry in read.filter_map(|entry| entry.ok()) {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                let size = sizes.get(&path).copied().unwrap_or(0);
                out.push(FileEntry {
                    path: path.display().to_string(),
                    size,
                    is_dir: true,
                });
            } else if file_type.is_file() {
                let size = entry.metadata().map(|m| on_disk_size(&m)).unwrap_or(0);
                out.push(FileEntry {
                    path: path.display().to_string(),
                    size,
                    is_dir: false,
                });
            }
        }
    }
    out.sort_by(|a, b| b.size.cmp(&a.size));
    Ok(out)
}

/// Subtract trashed items from the index so directory totals stay roughly
/// correct without a rescan. `items` is (path, size as shown in the UI).
pub fn forget(state: &TreeState, items: &[(String, u64)]) {
    let mut sizes = state.0.dir_sizes.lock().unwrap();
    for (path, size) in items {
        let path = Path::new(path);
        for ancestor in path.ancestors().skip(1) {
            match sizes.get_mut(ancestor) {
                Some(total) => *total = total.saturating_sub(*size),
                None => break, // reached above the scanned root
            }
        }
        sizes.retain(|key, _| !key.starts_with(path));
    }
}
