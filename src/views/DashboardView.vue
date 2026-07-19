<script setup lang="ts">
import { onMounted, ref } from "vue";
import { formatBytes, getDiskInfo } from "../api";
import type { DiskInfo } from "../types";

const emit = defineEmits<{
  analyze: [path: string];
}>();

const disks = ref<DiskInfo[]>([]);
const error = ref("");

onMounted(async () => {
  try {
    disks.value = await getDiskInfo();
  } catch (e) {
    error.value = String(e);
  }
});

function usedPercent(d: DiskInfo): number {
  if (d.total === 0) return 0;
  return Math.round(((d.total - d.available) / d.total) * 100);
}
</script>

<template>
  <div>
    <h1 class="text-2xl font-semibold">Tổng quan</h1>
    <p class="mt-1 text-sm text-zinc-400">
      Dung lượng các ổ đĩa trên máy của bạn.
    </p>

    <p v-if="error" class="mt-6 text-sm text-red-400">{{ error }}</p>

    <div class="mt-6 grid grid-cols-1 gap-4 lg:grid-cols-2">
      <div
        v-for="d in disks"
        :key="d.mount_point"
        class="cursor-pointer rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5 transition-colors hover:border-emerald-700/60 hover:bg-zinc-900"
        title="Phân tích dung lượng ổ này"
        @click="emit('analyze', d.mount_point)"
      >
        <div class="flex items-baseline justify-between">
          <div>
            <div class="font-medium text-zinc-100">
              {{ d.name || d.mount_point }}
            </div>
            <div class="mt-0.5 font-mono text-xs text-zinc-500">
              {{ d.mount_point }} · {{ d.file_system }}
            </div>
          </div>
          <div class="text-right">
            <div class="text-lg font-semibold">
              {{ formatBytes(d.total - d.available) }}
            </div>
            <div class="text-xs text-zinc-500">
              đã dùng / {{ formatBytes(d.total) }}
            </div>
          </div>
        </div>
        <div class="mt-4 h-2 overflow-hidden rounded-full bg-zinc-800">
          <div
            class="h-full rounded-full transition-all"
            :class="usedPercent(d) > 90 ? 'bg-red-500' : usedPercent(d) > 75 ? 'bg-amber-500' : 'bg-emerald-500'"
            :style="{ width: usedPercent(d) + '%' }"
          />
        </div>
        <div class="mt-2 flex items-center justify-between text-xs text-zinc-500">
          <span>
            Còn trống {{ formatBytes(d.available) }} ({{ 100 - usedPercent(d) }}%)
          </span>
          <span class="text-emerald-600">Phân tích ›</span>
        </div>
      </div>
    </div>
  </div>
</template>
