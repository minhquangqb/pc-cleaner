use serde::Serialize;
use sysinfo::Disks;

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total: u64,
    pub available: u64,
    pub file_system: String,
}

pub fn disk_info() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();
    let mut seen = std::collections::HashSet::new();
    disks
        .list()
        .iter()
        .filter_map(|d| {
            let mount = d.mount_point().display().to_string();
            // On macOS the same volume shows up multiple times; keep unique mounts
            // and skip system-internal read-only mounts.
            if !seen.insert(mount.clone()) {
                return None;
            }
            if mount.starts_with("/System/Volumes/") && mount != "/System/Volumes/Data" {
                return None;
            }
            Some(DiskInfo {
                name: d.name().to_string_lossy().to_string(),
                mount_point: mount,
                total: d.total_space(),
                available: d.available_space(),
                file_system: d.file_system().to_string_lossy().to_string(),
            })
        })
        .collect()
}
