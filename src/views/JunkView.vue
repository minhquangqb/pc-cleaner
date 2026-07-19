<script setup lang="ts">
import { computed, ref } from "vue";
import { ChevronDown, ChevronRight, CircleCheck } from "@lucide/vue";
import { cleanPaths, formatBytes, scanJunk } from "../api";
import type { JunkCategory } from "../types";
import ConfirmClean from "../components/ConfirmClean.vue";
import ScanStatus from "../components/ScanStatus.vue";
import { useScanProgress } from "../composables/useScanProgress";

const { progress, reset: resetProgress } = useScanProgress("junk");
const categories = ref<JunkCategory[]>([]);
const scanning = ref(false);
const scanned = ref(false);
const cleaning = ref(false);
const confirming = ref(false);
const error = ref("");
const lastFreed = ref<number | null>(null);
const selected = ref<Set<string>>(new Set());
const expanded = ref<Set<string>>(new Set());

const sizeByPath = computed(() => {
  const map = new Map<string, number>();
  for (const cat of categories.value)
    for (const e of cat.entries) map.set(e.path, e.size);
  return map;
});

const selectedPaths = computed(() => [...selected.value]);
const selectedSize = computed(() =>
  selectedPaths.value.reduce((sum, p) => sum + (sizeByPath.value.get(p) ?? 0), 0),
);
const totalSize = computed(() =>
  categories.value.reduce((sum, c) => sum + c.total_size, 0),
);

async function runScan() {
  scanning.value = true;
  error.value = "";
  lastFreed.value = null;
  selected.value = new Set();
  resetProgress();
  try {
    categories.value = await scanJunk();
    scanned.value = true;
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

function toggleCategory(cat: JunkCategory) {
  const next = new Set(selected.value);
  const allSelected = cat.entries.every((e) => next.has(e.path));
  for (const e of cat.entries)
    allSelected ? next.delete(e.path) : next.add(e.path);
  selected.value = next;
}

function toggleExpand(id: string) {
  const next = new Set(expanded.value);
  next.has(id) ? next.delete(id) : next.add(id);
  expanded.value = next;
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
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold">Dọn rác hệ thống</h1>
        <p class="mt-1 text-sm text-zinc-400">
          Cache, log và file tạm — an toàn để xóa, sẽ được chuyển vào Thùng rác.
        </p>
      </div>
      <button
        class="rounded-xl bg-emerald-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
        :disabled="scanning"
        @click="runScan"
      >
        {{ scanning ? "Đang quét..." : scanned ? "Quét lại" : "Quét ngay" }}
      </button>
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

    <ScanStatus
      v-if="scanning"
      :progress="progress"
      fallback="Đang tính dung lượng các thư mục cache..."
    />

    <template v-if="scanned && !scanning">
      <div class="mt-4 text-sm text-zinc-400">
        Tìm thấy tổng cộng
        <span class="font-semibold text-zinc-100">{{ formatBytes(totalSize) }}</span>
        có thể dọn.
      </div>

      <div class="mt-4 space-y-3">
        <div
          v-for="cat in categories"
          :key="cat.id"
          class="rounded-2xl border border-zinc-800 bg-zinc-900/60"
        >
          <div class="flex items-center gap-3 p-4">
            <input
              type="checkbox"
              class="size-4 accent-emerald-500"
              :checked="cat.entries.every((e) => selected.has(e.path))"
              @change="toggleCategory(cat)"
            />
            <button
              class="flex flex-1 items-center justify-between text-left"
              @click="toggleExpand(cat.id)"
            >
              <div>
                <div class="font-medium">{{ cat.name }}</div>
                <div class="text-xs text-zinc-500">{{ cat.description }}</div>
              </div>
              <div class="flex items-center gap-3">
                <span class="text-sm font-semibold text-zinc-200">
                  {{ formatBytes(cat.total_size) }}
                </span>
                <component
                  :is="expanded.has(cat.id) ? ChevronDown : ChevronRight"
                  class="size-4 text-zinc-500"
                />
              </div>
            </button>
          </div>

          <div v-if="expanded.has(cat.id)" class="border-t border-zinc-800">
            <div
              v-for="e in cat.entries"
              :key="e.path"
              class="flex items-center gap-3 px-4 py-2 hover:bg-zinc-800/40"
            >
              <input
                type="checkbox"
                class="size-4 accent-emerald-500"
                :checked="selected.has(e.path)"
                @change="toggleEntry(e.path)"
              />
              <span class="flex-1 truncate font-mono text-xs text-zinc-400">
                {{ e.path }}
              </span>
              <span class="text-xs text-zinc-300">{{ formatBytes(e.size) }}</span>
            </div>
          </div>
        </div>
      </div>

      <div
        v-if="selected.size > 0"
        class="sticky bottom-4 mt-6 flex items-center justify-between rounded-2xl border border-zinc-700 bg-zinc-900 p-4 shadow-xl"
      >
        <span class="text-sm text-zinc-300">
          Đã chọn {{ selected.size }} mục ·
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
