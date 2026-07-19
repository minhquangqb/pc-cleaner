export interface FileEntry {
  path: string;
  size: number;
  is_dir: boolean;
}

export interface JunkCategory {
  id: string;
  name: string;
  description: string;
  total_size: number;
  entries: FileEntry[];
}

export interface DupGroup {
  hash: string;
  size: number;
  wasted: number;
  paths: string[];
}

export interface DiskInfo {
  name: string;
  mount_point: string;
  total: number;
  available: number;
  file_system: string;
}

export interface CleanResult {
  freed: number;
  errors: string[];
}

export interface ScanProgress {
  task: "junk" | "large" | "dupes";
  phase: string;
  detail: string;
  done: number;
  total: number;
}
