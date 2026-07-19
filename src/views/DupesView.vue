<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { cleanPaths, formatBytes, getHomeDir, scanDuplicates } from "../api";
import type { DupGroup } from "../types";
import ConfirmClean from "../components/ConfirmClean.vue";
import ScanStatus from "../components/ScanStatus.vue";
import { useScanProgress } from "../composables/useScanProgress";

const { progress, reset: resetProgress } = useScanProgress("dupes");
const root = ref("");
const minSizeKb = ref(1024);
const groups = ref<DupGroup[]>([]);
const scanning = ref(false);
const scanned = ref(false);
const cleaning = ref(false);
const confirming = ref(false);
const error = ref("");
const lastFreed = ref<number | null>(null);
const selected = ref<Set<string>>(new Set());

onMounted(async () => {
  root.value = await getHomeDir();
});

const totalWasted = computed(() =>
  groups.value.reduce((s, g) => s + g.wasted, 0),
);
const selectedPaths = computed(() => [...selected.value]);
const selectedSize = computed(() => {
  const sizes = new Map<string, number>();
  for (const g of groups.value) for (const p of g.paths) sizes.set(p, g.size);
  return selectedPaths.value.reduce((s, p) => s + (sizes.get(p) ?? 0), 0);
});

async function pickFolder() {
  const dir = await open({ directory: true, defaultPath: root.value });
  if (typeof dir === "string") root.value = dir;
}

async function runScan() {
  scanning.value = true;
  error.value = "";
  lastFreed.value = null;
  selected.value = new Set();
  resetProgress();
  try {
    groups.value = await scanDuplicates(root.value, minSizeKb.value);
    scanned.value = true;
  } catch (e) {
    error.value = String(e);
  } finally {
    scanning.value = false;
  }
}

function toggle(path: string) {
  const next = new Set(selected.value);
  next.has(path) ? next.delete(path) : next.add(path);
  selected.value = next;
}

/// Select every copy except the first in each group.
function selectAllButFirst() {
  const next = new Set<string>();
  for (const g of groups.value) for (const p of g.paths.slice(1)) next.add(p);
  selected.value = next;
}

async function doClean() {
  cleaning.value = true;
  try {
    const result = await cleanPaths(selectedPaths.value);
    lastFreed.value = result.freed;
    if (result.errors.length) {
      error.value = `Một số mục không xóa được:\n${result.errors.slice(0, 5).join("\n")}`;
    }
    confirming.value = false;
    await runScan();
  } catch (e) {
    error.value = String(e);
  } finally {
    cleaning.value = false;
  }
}
</script>

<template>
  <div>
    <h1 class="text-2xl font-semibold">File trùng lặp</h1>
    <p class="mt-1 text-sm text-zinc-400">
      Tìm các file có nội dung giống hệt nhau (so sánh bằng hash BLAKE3).
    </p>

    <div class="mt-5 flex flex-wrap items-center gap-3">
      <button
        class="max-w-md truncate rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2.5 text-left font-mono text-xs text-zinc-300 hover:border-zinc-500"
        @click="pickFolder"
      >
        📁 {{ root || "Chọn thư mục..." }}
      </button>
      <label class="flex items-center gap-2 text-sm text-zinc-400">
        Tối thiểu
        <input
          v-model.number="minSizeKb"
          type="number"
          min="1"
          class="w-24 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
        />
        KB
      </label>
      <button
        class="rounded-xl bg-emerald-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
        :disabled="scanning || !root"
        @click="runScan"
      >
        {{ scanning ? "Đang quét..." : "Quét" }}
      </button>
    </div>

    <div
      v-if="lastFreed !== null"
      class="mt-4 rounded-xl border border-emerald-800 bg-emerald-950/50 px-4 py-3 text-sm text-emerald-300"
    >
      ✓ Đã giải phóng {{ formatBytes(lastFreed) }} (chuyển vào Thùng rác).
    </div>
    <p v-if="error" class="mt-4 whitespace-pre-line text-sm text-red-400">
      {{ error }}
    </p>

    <ScanStatus
      v-if="scanning"
      :progress="progress"
      :fallback="`Đang so sánh nội dung file trong ${root}...`"
    />

    <template v-if="scanned && !scanning">
      <p v-if="groups.length === 0" class="mt-6 text-sm text-zinc-500">
        Không tìm thấy file trùng lặp nào.
      </p>
      <template v-else>
        <div class="mt-4 flex items-center justify-between">
          <span class="text-sm text-zinc-400">
            {{ groups.length }} nhóm trùng lặp · lãng phí
            <span class="font-semibold text-zinc-100">{{ formatBytes(totalWasted) }}</span>
          </span>
          <button
            class="rounded-lg border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:border-zinc-500"
            @click="selectAllButFirst"
          >
            Chọn tất cả, giữ lại bản đầu tiên
          </button>
        </div>

        <div class="mt-4 space-y-3">
          <div
            v-for="g in groups"
            :key="g.hash"
            class="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-4"
          >
            <div class="flex items-center justify-between text-xs text-zinc-500">
              <span>{{ g.paths.length }} bản sao · {{ formatBytes(g.size) }}/file</span>
              <span class="text-amber-400">
                lãng phí {{ formatBytes(g.wasted) }}
              </span>
            </div>
            <div class="mt-2 space-y-1">
              <div
                v-for="p in g.paths"
                :key="p"
                class="flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-zinc-800/40"
              >
                <input
                  type="checkbox"
                  class="size-4 accent-emerald-500"
                  :checked="selected.has(p)"
                  @change="toggle(p)"
                />
                <span class="flex-1 truncate font-mono text-xs text-zinc-400">
                  {{ p }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </template>

      <div
        v-if="selected.size > 0"
        class="sticky bottom-4 mt-6 flex items-center justify-between rounded-2xl border border-zinc-700 bg-zinc-900 p-4 shadow-xl"
      >
        <span class="text-sm text-zinc-300">
          Đã chọn {{ selected.size }} file ·
          <span class="font-semibold text-zinc-100">{{ formatBytes(selectedSize) }}</span>
        </span>
        <button
          class="rounded-xl bg-red-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-red-500"
          @click="confirming = true"
        >
          Dọn dẹp
        </button>
      </div>
    </template>

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
