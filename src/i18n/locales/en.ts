import type vi from "./vi";

type MessageSchema = typeof vi;

const en: MessageSchema = {
  app: {
    nav: {
      dashboard: "Overview",
      junk: "Junk Cleaner",
      dev: "Dev Projects",
      apps: "Uninstaller",
      large: "Large Files",
      tree: "Disk Usage",
      dupes: "Duplicates",
    },
    trashNote: "Every delete goes to the Trash — you can always restore.",
  },
  common: {
    scanning: "Scanning...",
    rescan: "Scan again",
    scanNow: "Scan now",
    scan: "Scan",
    clean: "Clean up",
    cancel: "Cancel",
    freed: "Freed {size} (moved to Trash).",
    cleanErrors: "Some items could not be removed:",
    selectedItems: "{count} item selected · | {count} items selected ·",
    selectedFiles: "{count} file selected · | {count} files selected ·",
    pickFolder: "Choose folder...",
    minSize: "Min",
  },
  time: {
    today: "today",
    daysAgo: "{n} day ago | {n} days ago",
    monthsAgo: "{n} month ago | {n} months ago",
    yearsAgo: "{n} year ago | {n} years ago",
  },
  confirm: {
    title: "Confirm cleanup",
    body: "{count} item(s) ({size}) will be moved to the Trash — you can restore them if needed.",
    busy: "Cleaning...",
    action: "Move to Trash",
  },
  scan: {
    filesCount: "{n} file | {n} files",
    itemsProgress: "{done} / {total} items",
    sizingCategory: "Calculating size: {name}",
    phases: {
      reading_apps: "Reading application info",
      dupes_stage1: "Stage 1/3 — listing files",
      dupes_stage2: "Stage 2/3 — comparing first 64KB",
      dupes_stage3: "Stage 3/3 — confirming with full hash",
      walking: "Walking directory tree",
      sorting: "Sorting results",
      sizing: "Calculating sizes",
      done: "Done",
      finding_projects: "Finding dev projects...",
      sizing_artifacts: "Calculating artifact sizes",
    },
  },
  dashboard: {
    title: "Overview",
    subtitle: "Storage usage of the drives on this machine.",
    analyzeHint: "Analyze this drive",
    used: "used / {total}",
    free: "{size} free ({percent}%)",
    analyze: "Analyze ›",
  },
  junk: {
    title: "System Junk",
    subtitle: "Caches, logs and temp files — safe to delete, moved to the Trash.",
    fallback: "Calculating cache folder sizes...",
    foundPrefix: "Found a total of",
    foundSuffix: "that can be cleaned.",
    categories: {
      app_caches: {
        name: "Application caches",
        description:
          "Caches of your applications — safe to delete, apps rebuild them when needed.",
      },
      logs: {
        name: "Log files",
        description: "Old application and system logs (user level).",
      },
      browser_caches: {
        name: "Browser caches",
        description:
          "Caches of Chrome, Firefox, Edge... History and passwords are not touched.",
      },
      dev_caches: {
        name: "Dev tool caches",
        description:
          "npm, pnpm, yarn, Cargo, Homebrew, Gradle, Xcode DerivedData — rebuilt on the next build/install.",
      },
      temp: {
        name: "Temporary files",
        description: "The user temp directory.",
      },
    },
  },
  dev: {
    title: "Dev Project Cleanup",
    subtitle:
      "Scans your home folder for node_modules, target (Rust) and virtualenvs — all can be recreated with install/build.",
    fallback: "Looking for dev projects in your home folder...",
    empty: "No build artifacts found in your home folder.",
    foundPrefix: "Found {count} artifact(s), a total of",
    foundSuffix: "that can be cleaned.",
    sortSize: "Size",
    sortAge: "Oldest",
    lastBuilt: "last built/installed {age}",
    kinds: {
      node_modules: {
        name: "node_modules — Node.js",
        description: "Reinstall with npm / pnpm / yarn install when you need it again.",
      },
      target: {
        name: "target — Rust",
        description: "Cargo rebuilds it from scratch on the next cargo build.",
      },
      venv: {
        name: "Virtualenv — Python",
        description: "Recreate with python -m venv and pip install.",
      },
    },
  },
  apps: {
    title: "Uninstall Apps",
    subtitle:
      "Remove apps together with their leftovers (caches, preferences, logs...) — everything goes to the Trash.",
    scanApps: "Scan apps",
    fallback: "Reading installed applications...",
    empty: "No applications found (this feature currently supports macOS).",
    searchPlaceholder: "Search apps...",
    sortSize: "Size",
    sortUnused: "Least used",
    lastOpened: "opened {age}",
    never: "—",
    uninstallBtn: "Remove...",
    panelTitle: "Uninstall {name}",
    panelSubtitle: "Choose what to move to the Trash.",
    findingLeftovers: "Looking for leftover files...",
    leftoversHeader: "Leftovers in ~/Library",
    noLeftovers: "No leftover files found.",
    itemsCount: "{count} item · | {count} items ·",
    uninstall: "Uninstall",
  },
  large: {
    title: "Large Files",
    subtitle: "Find the biggest files in a folder to delete or move elsewhere.",
    fallback: "Scanning {root}...",
    empty: "No files ≥ {min} MB in this folder.",
  },
  dupes: {
    title: "Duplicate Files",
    subtitle: "Find files with identical content (compared by BLAKE3 hash).",
    fallback: "Comparing file contents in {root}...",
    empty: "No duplicate files found.",
    groupsFound: "{count} duplicate group(s) · wasting",
    keepFirst: "Select all, keep the first copy",
    copies: "{count} copies · {size}/file",
    wasted: "wasting {size}",
  },
  tree: {
    title: "Disk Usage",
    subtitle:
      "Pick a drive or folder and browse right away — folder sizes are computed in the background and refined over time.",
    usedShort: "{size} used",
    home: "Home",
    otherFolder: "Other folder...",
    total: "Total:",
    computing: "Calculating sizes...",
    doneComputing: "Done",
    emptyDir: "Empty folder.",
    pickToStart: "Pick a drive or folder above to get started.",
  },
};

export default en;
