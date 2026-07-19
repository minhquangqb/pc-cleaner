use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const SCAN_PROGRESS_EVENT: &str = "scan://progress";

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    /// Which scanner emitted this: "junk" | "large" | "dupes" | "dev" | "apps".
    pub task: String,
    /// Human-readable phase label.
    pub phase: String,
    /// Current path or item being processed.
    pub detail: String,
    pub done: u64,
    /// 0 means indeterminate (unknown total).
    pub total: u64,
}

pub fn emit(app: &AppHandle, task: &str, phase: &str, detail: &str, done: u64, total: u64) {
    let _ = app.emit(
        SCAN_PROGRESS_EVENT,
        ScanProgress {
            task: task.to_string(),
            phase: phase.to_string(),
            detail: detail.to_string(),
            done,
            total,
        },
    );
}
