<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { CircleCheck, FolderOpen } from "@lucide/vue";
import { open } from "@tauri-apps/plugin-dialog";
import { cleanPaths, formatBytes, getHomeDir, scanLargeFiles } from "../api";
import type { FileEntry } from "../types";
import ConfirmClean from "../components/ConfirmClean.vue";
import ScanStatus from "../components/ScanStatus.vue";
import { useScanProgress } from "../composables/useScanProgress";

const { t } = useI18n();
const { progress, reset: resetProgress } = useScanProgress("large");
const root = ref("");
const minSizeMb = ref(100);
const files = ref<FileEntry[]>([]);
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

const selectedPaths = computed(() => [...selected.value]);
const selectedSize = computed(() => {
  const sizes = new Map(files.value.map((f) => [f.path, f.size]));
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
    files.value = await scanLargeFiles(root.value, minSizeMb.value);
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

async function doClean() {
  cleaning.value = true;
  try {
    const result = await cleanPaths(selectedPaths.value);
    lastFreed.value = result.freed;
    if (result.errors.length) {
      error.value = `${t("common.cleanErrors")}\n${result.errors.slice(0, 5).join("\n")}`;
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
    <h1 class="text-2xl font-semibold">{{ t("large.title") }}</h1>
    <p class="mt-1 text-sm text-zinc-400">
      {{ t("large.subtitle") }}
    </p>

    <div class="mt-5 flex flex-wrap items-center gap-3">
      <button
        class="max-w-md truncate rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2.5 text-left font-mono text-xs text-zinc-300 hover:border-zinc-500"
        @click="pickFolder"
      >
        <FolderOpen class="mr-1 inline size-4 align-[-2px]" />
        {{ root || t("common.pickFolder") }}
      </button>
      <label class="flex items-center gap-2 text-sm text-zinc-400">
        {{ t("common.minSize") }}
        <input
          v-model.number="minSizeMb"
          type="number"
          min="1"
          class="w-24 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100"
        />
        MB
      </label>
      <button
        class="rounded-xl bg-emerald-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
        :disabled="scanning || !root"
        @click="runScan"
      >
        {{ scanning ? t("common.scanning") : t("common.scan") }}
      </button>
    </div>

    <div
      v-if="lastFreed !== null"
      class="mt-4 rounded-xl border border-emerald-800 bg-emerald-950/50 px-4 py-3 text-sm text-emerald-300"
    >
      <CircleCheck class="mr-1 inline size-4 align-[-2px]" />
      {{ t("common.freed", { size: formatBytes(lastFreed) }) }}
    </div>
    <p v-if="error" class="mt-4 whitespace-pre-line text-sm text-red-400">
      {{ error }}
    </p>

    <ScanStatus
      v-if="scanning"
      :progress="progress"
      :fallback="t('large.fallback', { root })"
    />

    <template v-if="scanned && !scanning">
      <p v-if="files.length === 0" class="mt-6 text-sm text-zinc-500">
        {{ t("large.empty", { min: minSizeMb }) }}
      </p>
      <div v-else class="mt-5 overflow-hidden rounded-2xl border border-zinc-800">
        <div
          v-for="f in files"
          :key="f.path"
          class="flex items-center gap-3 border-b border-zinc-800/60 bg-zinc-900/60 px-4 py-2.5 last:border-b-0 hover:bg-zinc-800/40"
        >
          <input
            type="checkbox"
            class="size-4 accent-emerald-500"
            :checked="selected.has(f.path)"
            @change="toggle(f.path)"
          />
          <span class="flex-1 truncate font-mono text-xs text-zinc-400">
            {{ f.path }}
          </span>
          <span class="text-sm font-medium text-zinc-200">
            {{ formatBytes(f.size) }}
          </span>
        </div>
      </div>

      <div
        v-if="selected.size > 0"
        class="sticky bottom-4 mt-6 flex items-center justify-between rounded-2xl border border-zinc-700 bg-zinc-900 p-4 shadow-xl"
      >
        <span class="text-sm text-zinc-300">
          {{ t("common.selectedFiles", { count: selected.size }, selected.size) }}
          <span class="font-semibold text-zinc-100">{{ formatBytes(selectedSize) }}</span>
        </span>
        <button
          class="rounded-xl bg-red-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-red-500"
          @click="confirming = true"
        >
          {{ t("common.clean") }}
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
