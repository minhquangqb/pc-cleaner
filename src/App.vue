<script setup lang="ts">
import { ref, type Component } from "vue";
import {
  LayoutDashboard,
  Trash2,
  FolderCode,
  PackageX,
  FileBox,
  FolderTree,
  Files,
  Sparkles,
} from "@lucide/vue";
import { requestAnalyze } from "./composables/useAnalyzeTarget";
import DashboardView from "./views/DashboardView.vue";
import JunkView from "./views/JunkView.vue";
import DevJunkView from "./views/DevJunkView.vue";
import UninstallView from "./views/UninstallView.vue";
import LargeFilesView from "./views/LargeFilesView.vue";
import DupesView from "./views/DupesView.vue";
import TreeView from "./views/TreeView.vue";

interface NavItem {
  id: string;
  label: string;
  icon: Component;
  view: Component;
}

const nav: NavItem[] = [
  { id: "dashboard", label: "Tổng quan", icon: LayoutDashboard, view: DashboardView },
  { id: "junk", label: "Dọn rác", icon: Trash2, view: JunkView },
  { id: "dev", label: "Dự án dev", icon: FolderCode, view: DevJunkView },
  { id: "apps", label: "Gỡ ứng dụng", icon: PackageX, view: UninstallView },
  { id: "large", label: "File lớn", icon: FileBox, view: LargeFilesView },
  { id: "tree", label: "Phân tích", icon: FolderTree, view: TreeView },
  { id: "dupes", label: "Trùng lặp", icon: Files, view: DupesView },
];

const active = ref(nav[0]);

function openAnalyze(path: string) {
  requestAnalyze(path);
  active.value = nav.find((item) => item.id === "tree") ?? active.value;
}
</script>

<template>
  <div class="flex h-screen">
    <aside
      class="flex w-56 shrink-0 flex-col border-r border-zinc-800 bg-zinc-900/40 p-4"
    >
      <div class="flex items-center gap-2 px-2 py-3">
        <Sparkles class="size-5 text-emerald-400" />
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
          <component :is="item.icon" class="size-4 shrink-0" />
          {{ item.label }}
        </button>
      </nav>
      <div class="mt-auto px-2 text-[11px] leading-relaxed text-zinc-600">
        Mọi thao tác xóa đều chuyển vào Thùng rác — có thể khôi phục.
      </div>
    </aside>

    <main class="flex-1 overflow-y-auto p-8">
      <KeepAlive>
        <component :is="active.view" @analyze="openAnalyze" />
      </KeepAlive>
    </main>
  </div>
</template>
