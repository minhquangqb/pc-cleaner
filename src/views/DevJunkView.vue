<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { ChevronDown, ChevronRight, CircleCheck } from "@lucide/vue";
import { cleanPaths, formatBytes, getHomeDir, scanDevJunk } from "../api";
import type { DevArtifact } from "../types";
import ConfirmClean from "../components/ConfirmClean.vue";
import ScanStatus from "../components/ScanStatus.vue";
import { useScanProgress } from "../composables/useScanProgress";

const { t, te } = useI18n();

function kindMeta(kind: string): { name: string; description: string } {
  const key = `dev.kinds.${kind}`;
  return te(`${key}.name`)
    ? { name: t(`${key}.name`), description: t(`${key}.description`) }
    : { name: kind, description: "" };
}

const { progress, reset: resetProgress } = useScanProgress("dev");
const artifacts = ref<DevArtifact[]>([]);
const home = ref("");
const scanning = ref(false);
const scanned = ref(false);
const cleaning = ref(false);
const confirming = ref(false);
const error = ref("");
const lastFreed = ref<number | null>(null);
const selected = ref<Set<string>>(new Set());
const expanded = ref<Set<string>>(new Set());
const sortBy = ref<"size" | "age">("size");

onMounted(async () => {
  home.value = await getHomeDir();
});

const groups = computed(() => {
  const byKind = new Map<string, DevArtifact[]>();
  for (const a of artifacts.value) {
    const list = byKind.get(a.kind) ?? [];
    list.push(a);
    byKind.set(a.kind, list);
  }
  return [...byKind.entries()]
    .map(([kind, entries]) => ({
      kind,
      meta: kindMeta(kind),
      entries:
        sortBy.value === "age"
          ? [...entries].sort((a, b) => b.age_days - a.age_days)
          : [...entries].sort((a, b) => b.size - a.size),
      total: entries.reduce((sum, e) => sum + e.size, 0),
    }))
    .sort((a, b) => b.total - a.total);
});

const sizeByPath = computed(() => {
  const map = new Map<string, number>();
  for (const a of artifacts.value) map.set(a.path, a.size);
  return map;
});

const selectedPaths = computed(() => [...selected.value]);
const selectedSize = computed(() =>
  selectedPaths.value.reduce((sum, p) => sum + (sizeByPath.value.get(p) ?? 0), 0),
);
const totalSize = computed(() =>
  artifacts.value.reduce((sum, a) => sum + a.size, 0),
);

function shortProject(path: string): string {
  return home.value && path.startsWith(home.value)
    ? "~" + path.slice(home.value.length)
    : path;
}

function ageLabel(days: number): string {
  if (days === 0) return t("time.today");
  if (days >= 365) return t("time.yearsAgo", Math.floor(days / 365));
  if (days >= 30) return t("time.monthsAgo", Math.floor(days / 30));
  return t("time.daysAgo", days);
}

async function runScan() {
  scanning.value = true;
  error.value = "";
  lastFreed.value = null;
  selected.value = new Set();
  resetProgress();
  try {
    artifacts.value = await scanDevJunk();
    scanned.value = true;
    expanded.value = new Set(groups.value.map((g) => g.kind));
  } catch (e) {
    error.value = String(e);
  } finally {
    scanning.value = false;
  }
}

function toggleEntry(path: string) {
  const next = new Set(selected.value);
  next.has(path) ? next.delete(path) : next.add(path);
  selected.value = next;
}

function toggleGroup(entries: DevArtifact[]) {
  const next = new Set(selected.value);
  const allSelected = entries.every((e) => next.has(e.path));
  for (const e of entries)
    allSelected ? next.delete(e.path) : next.add(e.path);
  selected.value = next;
}

function toggleExpand(kind: string) {
  const next = new Set(expanded.value);
  next.has(kind) ? next.delete(kind) : next.add(kind);
  expanded.value = next;
}

async function doClean() {
  cleaning.value = true;
  try {
    const result = await cleanPaths(selectedPaths.value);
    lastFreed.value = result.freed;
    if (result.errors.length) {
      error.value = `${t("common.cleanErrors")}\n${result.errors.slice(0, 5).join("\n")}`;
    }
    // Drop cleaned artifacts locally instead of re-walking the whole home dir.
    const cleaned = new Set(selectedPaths.value);
    artifacts.value = artifacts.value.filter((a) => !cleaned.has(a.path));
    selected.value = new Set();
    confirming.value = false;
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
        <h1 class="text-2xl font-semibold">{{ t("dev.title") }}</h1>
        <p class="mt-1 text-sm text-zinc-400">
          {{ t("dev.subtitle") }}
        </p>
      </div>
      <button
        class="rounded-xl bg-emerald-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
        :disabled="scanning"
        @click="runScan"
      >
        {{ scanning ? t("common.scanning") : scanned ? t("common.rescan") : t("common.scanNow") }}
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
      :fallback="t('dev.fallback')"
    />

    <template v-if="scanned && !scanning">
      <div v-if="artifacts.length === 0" class="mt-8 text-sm text-zinc-500">
        {{ t("dev.empty") }}
      </div>

      <template v-else>
        <div class="mt-4 flex items-center justify-between">
          <div class="text-sm text-zinc-400">
            {{ t("dev.foundPrefix", { count: artifacts.length }) }}
            <span class="font-semibold text-zinc-100">{{ formatBytes(totalSize) }}</span>
            {{ t("dev.foundSuffix") }}
          </div>
          <div class="flex rounded-xl border border-zinc-800 p-0.5 text-xs">
            <button
              class="rounded-lg px-3 py-1.5"
              :class="sortBy === 'size' ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:text-zinc-300'"
              @click="sortBy = 'size'"
            >
              {{ t("dev.sortSize") }}
            </button>
            <button
              class="rounded-lg px-3 py-1.5"
              :class="sortBy === 'age' ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:text-zinc-300'"
              @click="sortBy = 'age'"
            >
              {{ t("dev.sortAge") }}
            </button>
          </div>
        </div>

        <div class="mt-4 space-y-3">
          <div
            v-for="group in groups"
            :key="group.kind"
            class="rounded-2xl border border-zinc-800 bg-zinc-900/60"
          >
            <div class="flex items-center gap-3 p-4">
              <input
                type="checkbox"
                class="size-4 accent-emerald-500"
                :checked="group.entries.every((e) => selected.has(e.path))"
                @change="toggleGroup(group.entries)"
              />
              <button
                class="flex flex-1 items-center justify-between text-left"
                @click="toggleExpand(group.kind)"
              >
                <div>
                  <div class="font-medium">{{ group.meta.name }}</div>
                  <div class="text-xs text-zinc-500">{{ group.meta.description }}</div>
                </div>
                <div class="flex items-center gap-3">
                  <span class="text-sm font-semibold text-zinc-200">
                    {{ formatBytes(group.total) }}
                  </span>
                  <component
                    :is="expanded.has(group.kind) ? ChevronDown : ChevronRight"
                    class="size-4 text-zinc-500"
                  />
                </div>
              </button>
            </div>

            <div v-if="expanded.has(group.kind)" class="border-t border-zinc-800">
              <div
                v-for="a in group.entries"
                :key="a.path"
                class="flex items-center gap-3 px-4 py-2 hover:bg-zinc-800/40"
              >
                <input
                  type="checkbox"
                  class="size-4 accent-emerald-500"
                  :checked="selected.has(a.path)"
                  @change="toggleEntry(a.path)"
                />
                <div class="min-w-0 flex-1">
                  <div class="truncate font-mono text-xs text-zinc-300">
                    {{ shortProject(a.project) }}
                  </div>
                  <div class="text-[11px] text-zinc-500">
                    {{ t("dev.lastBuilt", { age: ageLabel(a.age_days) }) }}
                  </div>
                </div>
                <span class="text-xs text-zinc-300">{{ formatBytes(a.size) }}</span>
              </div>
            </div>
          </div>
        </div>

        <div
          v-if="selected.size > 0"
          class="sticky bottom-4 mt-6 flex items-center justify-between rounded-2xl border border-zinc-700 bg-zinc-900 p-4 shadow-xl"
        >
          <span class="text-sm text-zinc-300">
            {{ t("common.selectedItems", { count: selected.size }, selected.size) }}
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
