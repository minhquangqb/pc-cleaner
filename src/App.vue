<script setup lang="ts">
import { computed, ref, type Component } from "vue";
import { useI18n } from "vue-i18n";
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
import { LOCALE_NAMES, SUPPORTED_LOCALES, setLocale, type Locale } from "./i18n";
import { getPlatform, setAppLanguage } from "./api";
import DashboardView from "./views/DashboardView.vue";
import JunkView from "./views/JunkView.vue";
import DevJunkView from "./views/DevJunkView.vue";
import UninstallView from "./views/UninstallView.vue";
import LargeFilesView from "./views/LargeFilesView.vue";
import DupesView from "./views/DupesView.vue";
import TreeView from "./views/TreeView.vue";

const { t, locale } = useI18n();

interface NavItem {
  id: string;
  icon: Component;
  view: Component;
}

const allNav: NavItem[] = [
  { id: "dashboard", icon: LayoutDashboard, view: DashboardView },
  { id: "junk", icon: Trash2, view: JunkView },
  { id: "dev", icon: FolderCode, view: DevJunkView },
  { id: "apps", icon: PackageX, view: UninstallView },
  { id: "large", icon: FileBox, view: LargeFilesView },
  { id: "tree", icon: FolderTree, view: TreeView },
  { id: "dupes", icon: Files, view: DupesView },
];

// The uninstaller only works on macOS — hide its tab elsewhere. Guess from
// the user agent for the first paint, then confirm with the backend.
const isMac = ref(/Mac/i.test(navigator.userAgent));
getPlatform().then((p) => (isMac.value = p === "macos"));

const nav = computed(() =>
  allNav.filter((item) => item.id !== "apps" || isMac.value),
);

const active = ref(allNav[0]);

function openAnalyze(path: string) {
  requestAnalyze(path);
  active.value = allNav.find((item) => item.id === "tree") ?? active.value;
}

function switchLocale(l: Locale) {
  setLocale(l);
  setAppLanguage(l).catch(() => {});
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
          {{ t(`app.nav.${item.id}`) }}
        </button>
      </nav>
      <div class="mt-auto space-y-3 px-2">
        <div class="flex rounded-lg border border-zinc-800 p-0.5 text-[11px]">
          <button
            v-for="l in SUPPORTED_LOCALES"
            :key="l"
            class="flex-1 rounded-md px-2 py-1 transition-colors"
            :class="
              locale === l
                ? 'bg-zinc-800 text-zinc-100'
                : 'text-zinc-500 hover:text-zinc-300'
            "
            @click="switchLocale(l)"
          >
            {{ LOCALE_NAMES[l] }}
          </button>
        </div>
        <div class="text-[11px] leading-relaxed text-zinc-600">
          {{ t("app.trashNote") }}
        </div>
      </div>
    </aside>

    <main class="flex-1 overflow-y-auto p-8">
      <KeepAlive>
        <component :is="active.view" @analyze="openAnalyze" />
      </KeepAlive>
    </main>
  </div>
</template>
