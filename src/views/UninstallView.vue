<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { CircleCheck, Search, X } from "@lucide/vue";
import { cleanPaths, findAppLeftovers, formatBytes, listApps } from "../api";
import type { AppInfo, FileEntry } from "../types";
import ConfirmClean from "../components/ConfirmClean.vue";
import ScanStatus from "../components/ScanStatus.vue";
import { useScanProgress } from "../composables/useScanProgress";

const { t } = useI18n();
const { progress, reset: resetProgress } = useScanProgress("apps");
const apps = ref<AppInfo[]>([]);
const scanning = ref(false);
const scanned = ref(false);
const search = ref("");
const sortBy = ref<"size" | "unused">("size");
const error = ref("");
const lastFreed = ref<number | null>(null);

// Uninstall panel state for the app being inspected.
const current = ref<AppInfo | null>(null);
const leftovers = ref<FileEntry[]>([]);
const loadingLeftovers = ref(false);
const checked = ref<Set<string>>(new Set());
const confirming = ref(false);
const cleaning = ref(false);

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase();
  const list = q
    ? apps.value.filter(
        (a) =>
          a.name.toLowerCase().includes(q) ||
          a.bundle_id.toLowerCase().includes(q),
      )
    : [...apps.value];
  if (sortBy.value === "size") list.sort((a, b) => b.size - a.size);
  else
    list.sort(
      (a, b) => (b.last_used_days ?? Infinity) - (a.last_used_days ?? Infinity),
    );
  return list;
});

function lastUsedLabel(app: AppInfo): string {
  const d = app.last_used_days;
  if (d === null) return t("apps.never");
  if (d === 0) return t("time.today");
  if (d >= 365) return t("time.yearsAgo", Math.floor(d / 365));
  if (d >= 30) return t("time.monthsAgo", Math.floor(d / 30));
  return t("time.daysAgo", d);
}

async function runScan() {
  scanning.value = true;
  error.value = "";
  lastFreed.value = null;
  resetProgress();
  try {
    apps.value = await listApps();
    scanned.value = true;
  } catch (e) {
    error.value = String(e);
  } finally {
    scanning.value = false;
  }
}

async function openApp(app: AppInfo) {
  current.value = app;
  leftovers.value = [];
  checked.value = new Set([app.path]);
  loadingLeftovers.value = true;
  try {
    leftovers.value = await findAppLeftovers(app.bundle_id, app.name);
    checked.value = new Set([app.path, ...leftovers.value.map((l) => l.path)]);
  } catch (e) {
    error.value = String(e);
  } finally {
    loadingLeftovers.value = false;
  }
}

function closePanel() {
  if (cleaning.value) return;
  current.value = null;
  leftovers.value = [];
  checked.value = new Set();
}

function toggleChecked(path: string) {
  const next = new Set(checked.value);
  next.has(path) ? next.delete(path) : next.add(path);
  checked.value = next;
}

const checkedPaths = computed(() => [...checked.value]);
const checkedSize = computed(() => {
  let sum = 0;
  const app = current.value;
  if (app && checked.value.has(app.path)) sum += app.size;
  for (const l of leftovers.value) if (checked.value.has(l.path)) sum += l.size;
  return sum;
});

async function doUninstall() {
  const app = current.value;
  if (!app) return;
  cleaning.value = true;
  try {
    const result = await cleanPaths(checkedPaths.value);
    lastFreed.value = result.freed;
    if (result.errors.length) {
      error.value = `${t("common.cleanErrors")}\n${result.errors.slice(0, 5).join("\n")}`;
    } else {
      error.value = "";
    }
    // Only drop the app from the list when its bundle was actually removed.
    const bundleFailed = result.errors.some((e) => e.includes(app.path));
    if (checked.value.has(app.path) && !bundleFailed) {
      apps.value = apps.value.filter((a) => a.path !== app.path);
    }
    confirming.value = false;
    // Reset before closePanel — its guard blocks closing while cleaning.
    cleaning.value = false;
    closePanel();
  } catch (e) {
    error.value = String(e);
  } finally {
    cleaning.value = false;
  }
}
</script>

<template>
  <div>
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">{{ t("apps.title") }}</h1>
        <p class="mt-1 text-sm text-zinc-400">
          {{ t("apps.subtitle") }}
        </p>
      </div>
      <button
        class="rounded-xl bg-emerald-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
        :disabled="scanning"
        @click="runScan"
      >
        {{ scanning ? t("common.scanning") : scanned ? t("common.rescan") : t("apps.scanApps") }}
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
      :fallback="t('apps.fallback')"
    />

    <template v-if="scanned && !scanning">
      <div v-if="apps.length === 0" class="mt-8 text-sm text-zinc-500">
        {{ t("apps.empty") }}
      </div>

      <template v-else>
        <div class="mt-5 flex items-center gap-3">
          <div class="relative flex-1">
            <Search
              class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-zinc-500"
            />
            <input
              v-model="search"
              type="text"
              :placeholder="t('apps.searchPlaceholder')"
              class="w-full rounded-xl border border-zinc-800 bg-zinc-900/60 py-2 pl-9 pr-3 text-sm text-zinc-200 placeholder:text-zinc-600 focus:border-emerald-700 focus:outline-none"
            />
          </div>
          <div class="flex rounded-xl border border-zinc-800 p-0.5 text-xs">
            <button
              class="rounded-lg px-3 py-1.5"
              :class="sortBy === 'size' ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:text-zinc-300'"
              @click="sortBy = 'size'"
            >
              {{ t("apps.sortSize") }}
            </button>
            <button
              class="rounded-lg px-3 py-1.5"
              :class="sortBy === 'unused' ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:text-zinc-300'"
              @click="sortBy = 'unused'"
            >
              {{ t("apps.sortUnused") }}
            </button>
          </div>
        </div>

        <div class="mt-4 overflow-hidden rounded-2xl border border-zinc-800">
          <div
            v-for="app in filtered"
            :key="app.path"
            class="flex items-center gap-4 border-b border-zinc-800/60 bg-zinc-900/60 px-4 py-3 last:border-b-0 hover:bg-zinc-800/40"
          >
            <img
              v-if="app.icon"
              :src="app.icon"
              class="size-8 shrink-0 rounded-lg"
              alt=""
            />
            <div
              v-else
              class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-zinc-800 text-sm font-semibold text-zinc-500"
            >
              {{ app.name.charAt(0).toUpperCase() }}
            </div>
            <div class="min-w-0 flex-1">
              <div class="truncate text-sm font-medium text-zinc-100">
                {{ app.name }}
              </div>
              <div class="truncate font-mono text-[11px] text-zinc-500">
                {{ app.bundle_id }}
              </div>
            </div>
            <div class="w-28 shrink-0 text-right text-xs text-zinc-400">
              {{ app.last_used_days === null ? t("apps.never") : t("apps.lastOpened", { age: lastUsedLabel(app) }) }}
            </div>
            <div class="w-20 shrink-0 text-right text-sm font-semibold text-zinc-200">
              {{ formatBytes(app.size) }}
            </div>
            <button
              class="shrink-0 rounded-lg border border-red-900/60 px-3 py-1.5 text-xs font-medium text-red-400 hover:bg-red-950/40"
              @click="openApp(app)"
            >
              {{ t("apps.uninstallBtn") }}
            </button>
          </div>
        </div>
      </template>
    </template>

    <!-- Leftover picker panel -->
    <div
      v-if="current"
      class="fixed inset-0 z-40 flex items-center justify-center bg-black/60"
      @click.self="closePanel"
    >
      <div
        class="mx-4 flex max-h-[80vh] w-full max-w-xl flex-col rounded-2xl border border-zinc-700 bg-zinc-900 p-6 shadow-2xl"
      >
        <div class="flex items-start justify-between">
          <div class="flex items-center gap-3">
            <img
              v-if="current.icon"
              :src="current.icon"
              class="size-10 shrink-0 rounded-xl"
              alt=""
            />
            <div>
              <h2 class="text-lg font-semibold text-zinc-100">
                {{ t("apps.panelTitle", { name: current.name }) }}
              </h2>
              <p class="mt-1 text-sm text-zinc-400">
                {{ t("apps.panelSubtitle") }}
              </p>
            </div>
          </div>
          <button
            class="rounded-lg p-1.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300"
            @click="closePanel"
          >
            <X class="size-4" />
          </button>
        </div>

        <div class="mt-4 flex-1 space-y-1 overflow-y-auto rounded-lg bg-zinc-950/60 p-3">
          <label class="flex cursor-pointer items-center gap-3 rounded-md px-2 py-1.5 hover:bg-zinc-800/40">
            <input
              type="checkbox"
              class="size-4 accent-emerald-500"
              :checked="checked.has(current.path)"
              @change="toggleChecked(current.path)"
            />
            <span class="flex-1 truncate font-mono text-xs text-zinc-300">
              {{ current.path }}
            </span>
            <span class="text-xs text-zinc-300">{{ formatBytes(current.size) }}</span>
          </label>

          <div v-if="loadingLeftovers" class="px-2 py-2 text-xs text-zinc-500">
            {{ t("apps.findingLeftovers") }}
          </div>
          <template v-else>
            <div
              v-if="leftovers.length"
              class="px-2 pt-2 text-[11px] font-medium uppercase tracking-wide text-zinc-600"
            >
              {{ t("apps.leftoversHeader") }}
            </div>
            <label
              v-for="l in leftovers"
              :key="l.path"
              class="flex cursor-pointer items-center gap-3 rounded-md px-2 py-1.5 hover:bg-zinc-800/40"
            >
              <input
                type="checkbox"
                class="size-4 accent-emerald-500"
                :checked="checked.has(l.path)"
                @change="toggleChecked(l.path)"
              />
              <span class="flex-1 truncate font-mono text-xs text-zinc-400">
                {{ l.path }}
              </span>
              <span class="text-xs text-zinc-400">{{ formatBytes(l.size) }}</span>
            </label>
            <div
              v-if="!leftovers.length"
              class="px-2 py-2 text-xs text-zinc-500"
            >
              {{ t("apps.noLeftovers") }}
            </div>
          </template>
        </div>

        <div class="mt-5 flex items-center justify-between">
          <span class="text-sm text-zinc-400">
            {{ t("apps.itemsCount", { count: checkedPaths.length }, checkedPaths.length) }}
            <span class="font-semibold text-zinc-200">{{ formatBytes(checkedSize) }}</span>
          </span>
          <div class="flex gap-3">
            <button
              class="rounded-lg px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-800"
              @click="closePanel"
            >
              {{ t("common.cancel") }}
            </button>
            <button
              class="rounded-lg bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-500 disabled:opacity-50"
              :disabled="checkedPaths.length === 0 || loadingLeftovers"
              @click="confirming = true"
            >
              {{ t("apps.uninstall") }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <ConfirmClean
      v-if="confirming"
      :paths="checkedPaths"
      :total-size="checkedSize"
      :busy="cleaning"
      @confirm="doUninstall"
      @cancel="confirming = false"
    />
  </div>
</template>
