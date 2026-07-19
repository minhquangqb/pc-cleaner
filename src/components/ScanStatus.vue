<script setup lang="ts">
import { computed } from "vue";
import type { ScanProgress } from "../types";

const props = defineProps<{
  progress: ScanProgress | null;
  fallback: string;
}>();

const percent = computed(() => {
  const p = props.progress;
  if (!p || p.total === 0) return null;
  return Math.min(100, Math.round((p.done / p.total) * 100));
});
</script>

<template>
  <div class="mt-8 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">
    <div class="flex items-center gap-3">
      <span
        class="size-5 shrink-0 animate-spin rounded-full border-2 border-zinc-600 border-t-emerald-500"
      />
      <span class="text-sm font-medium text-zinc-200">
        {{ props.progress?.phase ?? props.fallback }}
      </span>
      <span v-if="percent !== null" class="ml-auto text-sm font-semibold text-emerald-400">
        {{ percent }}%
      </span>
      <span
        v-else-if="props.progress && props.progress.done > 0"
        class="ml-auto text-sm text-zinc-400"
      >
        {{ props.progress.done.toLocaleString() }} file
      </span>
    </div>

    <div v-if="percent !== null" class="mt-3 h-1.5 overflow-hidden rounded-full bg-zinc-800">
      <div
        class="h-full rounded-full bg-emerald-500 transition-all duration-200"
        :style="{ width: percent + '%' }"
      />
    </div>

    <div
      v-if="props.progress?.detail"
      class="mt-3 truncate font-mono text-xs text-zinc-500"
    >
      {{ props.progress.detail }}
    </div>
    <div
      v-if="props.progress && props.progress.total > 0"
      class="mt-1 text-xs text-zinc-600"
    >
      {{ props.progress.done.toLocaleString() }} /
      {{ props.progress.total.toLocaleString() }} mục
    </div>
  </div>
</template>
