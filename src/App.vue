<script setup lang="ts">
import { ref, type Component } from "vue";
import DashboardView from "./views/DashboardView.vue";
import JunkView from "./views/JunkView.vue";
import LargeFilesView from "./views/LargeFilesView.vue";
import DupesView from "./views/DupesView.vue";

interface NavItem {
  id: string;
  label: string;
  icon: string;
  view: Component;
}

const nav: NavItem[] = [
  { id: "dashboard", label: "Tổng quan", icon: "📊", view: DashboardView },
  { id: "junk", label: "Dọn rác", icon: "🧹", view: JunkView },
  { id: "large", label: "File lớn", icon: "📦", view: LargeFilesView },
  { id: "dupes", label: "Trùng lặp", icon: "🗂️", view: DupesView },
];

const active = ref(nav[0]);
</script>

<template>
  <div class="flex h-screen">
    <aside
      class="flex w-56 shrink-0 flex-col border-r border-zinc-800 bg-zinc-900/40 p-4"
    >
      <div class="flex items-center gap-2 px-2 py-3">
        <span class="text-xl">✨</span>
        <span class="font-semibold tracking-tight">PC Cleaner</span>
      </div>
      <nav class="mt-4 space-y-1">
        <button
          v-for="item in nav"
          :key="item.id"
          class="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-sm transition-colors"
          :class="
            active.id === item.id
              ? 'bg-emerald-600/15 font-medium text-emerald-400'
              : 'text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-200'
          "
          @click="active = item"
        >
          <span>{{ item.icon }}</span>
          {{ item.label }}
        </button>
      </nav>
      <div class="mt-auto px-2 text-[11px] leading-relaxed text-zinc-600">
        Mọi thao tác xóa đều chuyển vào Thùng rác — có thể khôi phục.
      </div>
    </aside>

    <main class="flex-1 overflow-y-auto p-8">
      <KeepAlive>
        <component :is="active.view" />
      </KeepAlive>
    </main>
  </div>
</template>
