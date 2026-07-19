<script setup lang="ts">
import { formatBytes } from "../api";

const props = defineProps<{
  paths: string[];
  totalSize: number;
  busy: boolean;
}>();

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
    @click.self="emit('cancel')"
  >
    <div
      class="mx-4 flex max-h-[80vh] w-full max-w-xl flex-col rounded-2xl border border-zinc-700 bg-zinc-900 p-6 shadow-2xl"
    >
      <h2 class="text-lg font-semibold text-zinc-100">Xác nhận dọn dẹp</h2>
      <p class="mt-1 text-sm text-zinc-400">
        {{ props.paths.length }} mục ({{ formatBytes(props.totalSize) }}) sẽ được
        chuyển vào <span class="font-medium text-zinc-200">Thùng rác</span> —
        bạn có thể khôi phục nếu cần.
      </p>

      <ul
        class="mt-4 flex-1 space-y-1 overflow-y-auto rounded-lg bg-zinc-950/60 p-3 text-xs text-zinc-400"
      >
        <li v-for="p in props.paths" :key="p" class="truncate font-mono">
          {{ p }}
        </li>
      </ul>

      <div class="mt-5 flex justify-end gap-3">
        <button
          class="rounded-lg px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-800"
          :disabled="props.busy"
          @click="emit('cancel')"
        >
          Hủy
        </button>
        <button
          class="rounded-lg bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-500 disabled:opacity-50"
          :disabled="props.busy"
          @click="emit('confirm')"
        >
          {{ props.busy ? "Đang dọn..." : "Chuyển vào Thùng rác" }}
        </button>
      </div>
    </div>
  </div>
</template>
