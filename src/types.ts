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

export interface DevArtifact {
  path: string;
  project: string;
  kind: string;
  size: number;
  age_days: number;
}

export interface AppInfo {
  path: string;
  name: string;
  bundle_id: string;
  size: number;
  last_used_days: number | null;
}

export interface ScanProgress {
  task: "junk" | "large" | "dupes" | "tree" | "dev" | "apps";
  phase: string;
  detail: string;
  done: number;
  total: number;
}
