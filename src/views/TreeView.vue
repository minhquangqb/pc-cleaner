<script setup lang="ts">
import { computed, nextTick, onActivated, onMounted, onUnmounted, ref } from "vue";
import { CircleCheck, FolderOpen, HardDrive, House } from "@lucide/vue";
import { open } from "@tauri-apps/plugin-dialog";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  cleanPaths,
  forgetTreePaths,
  formatBytes,
  getDiskInfo,
  getHomeDir,
  getTreeChildren,
  setTreeFocus,
  startTreeScan,
} from "../api";
import type { DiskInfo, FileEntry } from "../types";
import ConfirmClean from "../components/ConfirmClean.vue";
import { consumeAnalyzeTarget } from "../composables/useAnalyzeTarget";
import { useScanProgress } from "../composables/useScanProgress";

const TREE_DONE_EVENT = "tree://done";
const REFRESH_MS = 700;

interface Column {
  path: string;
  entries: FileEntry[];
}

const { progress, reset: resetProgress } = useScanProgress("tree");
const disks = ref<DiskInfo[]>([]);
const home = ref("");
const root = ref("");
const scanning = ref(false);
const cleaning = ref(false);
const confirming = ref(false);
const error = ref("");
const lastFreed = ref<number | null>(null);
const columns = ref<Column[]>([]);
const drillPath = ref<string[]>([]);
const selected = ref<Set<string>>(new Set());
const columnsEl = ref<HTMLElement | null>(null);

let timer: number | undefined;
let refreshing = false;
let doneUnlisten: UnlistenFn | undefined;

onMounted(async () => {
  home.value = await getHomeDir();
  disks.value = await getDiskInfo();
  doneUnlisten = await listen<number>(TREE_DONE_EVENT, async () => {
    scanning.value = false;
    stopPolling();
    await refreshColumns();
  });
});

onUnmounted(() => {
  stopPolling();
  doneUnlisten?.();
});

// Fires on first mount and every tab re-activation (KeepAlive).
onActivated(() => {
  const path = consumeAnalyzeTarget();
  if (path) startScan(path);
});

const totalSize = computed(
  () => columns.value[0]?.entries.reduce((sum, e) => sum + e.size, 0) ?? 0,
);
const sizeByPath = computed(() => {
  const map = new Map<string, number>();
  for (const col of columns.value)
    for (const e of col.entries) map.set(e.path, e.size);
  return map;
});
const selectedPaths = computed(() => [...selected.value]);
const selectedSize = computed(() =>
  selectedPaths.value.reduce((s, p) => s + (sizeByPath.value.get(p) ?? 0), 0),
);

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

function sizeLabel(entry: FileEntry): string {
  const label = formatBytes(entry.size);
  return scanning.value && entry.is_dir ? `${label}+` : label;
}

function startPolling() {
  stopPolling();
  timer = window.setInterval(refreshColumns, REFRESH_MS);
}

function stopPolling() {
  if (timer !== undefined) {
    clearInterval(timer);
    timer = undefined;
  }
}

async function refreshColumns() {
  if (refreshing || columns.value.length === 0) return;
  refreshing = true;
  try {
    const results = await Promise.all(
      columns.value.map((col) => getTreeChildren(col.path).catch(() => null)),
    );
    columns.value = columns.value.map((col, i) =>
      results[i] ? { path: col.path, entries: results[i]! } : col,
    );
  } finally {
    refreshing = false;
  }
}

async function startScan(path: string) {
  root.value = path;
  error.value = "";
  lastFreed.value = null;
  selected.value = new Set();
  drillPath.value = [];
  columns.value = [];
  resetProgress();
  try {
    await startTreeScan(path);
    scanning.value = true;
    columns.value = [{ path, entries: await getTreeChildren(path) }];
    startPolling();
  } catch (e) {
    error.value = String(e);
    scanning.value = false;
  }
}

async function pickFolder() {
  const dir = await open({ directory: true, defaultPath: root.value || home.value });
  if (typeof dir === "string") await startScan(dir);
}

async function drill(colIndex: number, entry: FileEntry) {
  drillPath.value = [...drillPath.value.slice(0, colIndex), entry.path];
  columns.value = columns.value.slice(0, colIndex + 1);
  if (!entry.is_dir) return;
  try {
    // Prioritize sizing the subtree the user just opened.
    if (scanning.value) setTreeFocus(entry.path).catch(() => {});
    const entries = await getTreeChildren(entry.path);
    columns.value = [...columns.value, { path: entry.path, entries }];
    await nextTick();
    columnsEl.value?.scrollTo({
      left: columnsEl.value.scrollWidth,
      behavior: "smooth",
    });
  } catch (e) {
    error.value = String(e);
  }
}

function toggle(path: string) {
  const next = new Set(selected.value);
  next.has(path) ? next.delete(path) : next.add(path);
  selected.value = next;
}

async function doClean() {
  cleaning.value = true;
  try {
    const items: [string, number][] = selectedPaths.value.map((p) => [
      p,
      sizeByPath.value.get(p) ?? 0,
    ]);
    const result = await cleanPaths(selectedPaths.value);
    lastFreed.value = result.freed;
    if (result.errors.length) {
      error.value = `Một số mục không xóa được:\n${result.errors.slice(0, 5).join("\n")}`;
    }
    confirming.value = false;
    selected.value = new Set();
    await forgetTreePaths(items);
    await refreshColumns();
  } catch (e) {
    error.value = String(e);
  } finally {
    cleaning.value = false;
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <h1 class="text-2xl font-semibold">Phân tích dung lượng</h1>
    <p class="mt-1 text-sm text-zinc-400">
      Chọn ổ đĩa hoặc thư mục rồi duyệt ngay — dung lượng thư mục được tính nền
      và cập nhật dần.
    </p>

    <div class="mt-5 flex flex-wrap items-center gap-2">
      <button
        v-for="d in disks"
        :key="d.mount_point"
        class="rounded-xl border px-4 py-2.5 text-sm transition-colors"
        :class="
          root === d.mount_point
            ? 'border-emerald-600 bg-emerald-600/15 text-emerald-300'
            : 'border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500'
        "
        @click="startScan(d.mount_point)"
      >
        <HardDrive class="mr-1 inline size-4 align-[-2px]" />
        {{ d.name || d.mount_point }}
        <span class="ml-1 text-xs text-zinc-500">
          {{ formatBytes(d.total - d.available) }} đã dùng
        </span>
      </button>
      <button
        v-if="home"
        class="rounded-xl border px-4 py-2.5 text-sm transition-colors"
        :class="
          root === home
            ? 'border-emerald-600 bg-emerald-600/15 text-emerald-300'
            : 'border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500'
        "
        @click="startScan(home)"
      >
        <House class="mr-1 inline size-4 align-[-2px]" />
        Home
      </button>
      <button
        class="rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2.5 text-sm text-zinc-300 hover:border-zinc-500"
        @click="pickFolder"
      >
        <FolderOpen class="mr-1 inline size-4 align-[-2px]" />
        Thư mục khác...
      </button>
    </div>

    <div
      v-if="columns.length > 0"
      class="mt-3 flex items-center gap-3 text-sm text-zinc-400"
    >
      <span>
        Tổng:
        <span class="font-semibold text-zinc-100">
          {{ formatBytes(totalSize) }}{{ scanning ? "+" : "" }}
        </span>
      </span>
      <span v-if="scanning" class="flex items-center gap-2 text-xs text-zinc-500">
        <span
          class="size-3 animate-spin rounded-full border-2 border-zinc-600 border-t-emerald-500"
        />
        Đang tính dung lượng...
        <span v-if="progress" class="max-w-md truncate font-mono">
          {{ progress.detail }}
        </span>
      </span>
      <span v-else class="flex items-center gap-1 text-xs text-emerald-500">
        <CircleCheck class="size-3.5" />
        Đã tính xong
      </span>
    </div>

    <div
      v-if="lastFreed !== null"
      class="mt-4 rounded-xl border border-emerald-800 bg-emerald-950/50 px-4 py-3 text-sm text-emerald-300"
    >
      <CircleCheck class="mr-1 inline size-4 align-[-2px]" />
      Đã giải phóng {{ formatBytes(lastFreed) }} (chuyển vào Thùng rác).
    </div>
    <p v-if="error" class="mt-4 whitespace-pre-line text-sm text-red-400">
      {{ error }}
    </p>

    <div
      v-if="columns.length > 0"
      ref="columnsEl"
      class="mt-4 flex min-h-0 flex-1 divide-x divide-zinc-800 overflow-x-auto rounded-2xl border border-zinc-800 bg-zinc-900/60"
    >
      <div
        v-for="(col, i) in columns"
        :key="col.path"
        class="flex w-80 shrink-0 flex-col overflow-y-auto"
      >
        <p
          v-if="col.entries.length === 0"
          class="px-4 py-3 text-xs text-zinc-500"
        >
          Thư mục trống.
        </p>
        <button
          v-for="e in col.entries"
          :key="e.path"
          class="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-zinc-800/50"
          :class="
            drillPath[i] === e.path ? 'bg-emerald-600/15 text-emerald-300' : ''
          "
          @click="drill(i, e)"
        >
          <input
            type="checkbox"
            class="size-3.5 shrink-0 accent-emerald-500"
            :checked="selected.has(e.path)"
            @click.stop
            @change="toggle(e.path)"
          />
          <span
            class="w-20 shrink-0 text-right font-mono text-xs"
            :class="
              drillPath[i] === e.path ? 'text-emerald-300' : 'text-zinc-300'
            "
          >
            {{ sizeLabel(e) }}
          </span>
          <span class="flex-1 truncate text-sm text-zinc-200">
            {{ fileName(e.path) }}
          </span>
          <span v-if="e.is_dir" class="shrink-0 text-xs text-zinc-500">›</span>
        </button>
      </div>
    </div>
    <p v-else class="mt-10 text-center text-sm text-zinc-500">
      Chọn một ổ đĩa hoặc thư mục ở trên để bắt đầu.
    </p>

    <div
      v-if="selected.size > 0"
      class="sticky bottom-4 mt-6 flex items-center justify-between rounded-2xl border border-zinc-700 bg-zinc-900 p-4 shadow-xl"
    >
      <span class="text-sm text-zinc-300">
        Đã chọn {{ selected.size }} mục ·
        <span class="font-semibold text-zinc-100">{{
          formatBytes(selectedSize)
        }}</span>
      </span>
      <button
        class="rounded-xl bg-red-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-red-500"
        @click="confirming = true"
      >
        Dọn dẹp
      </button>
    </div>

    <ConfirmClean
      v-if="confirming"
      :paths="selectedPaths"
      :total-size="selectedSize"
      :busy="cleaning"
      @confirm="doClean"
      @cancel="confirming = false"
    />
  </div>
</template>
