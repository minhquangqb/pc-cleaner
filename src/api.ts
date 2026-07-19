import { invoke } from "@tauri-apps/api/core";
import type {
  CleanResult,
  DiskInfo,
  DupGroup,
  FileEntry,
  JunkCategory,
} from "./types";

export const getDiskInfo = () => invoke<DiskInfo[]>("get_disk_info");

export const scanJunk = () => invoke<JunkCategory[]>("scan_junk");

export const scanLargeFiles = (root: string, minSizeMb: number, limit = 200) =>
  invoke<FileEntry[]>("scan_large_files", { root, minSizeMb, limit });

export const scanDuplicates = (root: string, minSizeKb: number) =>
  invoke<DupGroup[]>("scan_duplicates", { root, minSizeKb });

export const startTreeScan = (root: string) =>
  invoke<void>("start_tree_scan", { root });

export const getTreeChildren = (path: string) =>
  invoke<FileEntry[]>("get_tree_children", { path });

export const setTreeFocus = (path: string) =>
  invoke<void>("set_tree_focus", { path });

export const forgetTreePaths = (items: [string, number][]) =>
  invoke<void>("forget_tree_paths", { items });

export const cleanPaths = (paths: string[]) =>
  invoke<CleanResult>("clean_paths", { paths });

export const getHomeDir = () => invoke<string>("get_home_dir");

export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** i;
  return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${units[i]}`;
}
