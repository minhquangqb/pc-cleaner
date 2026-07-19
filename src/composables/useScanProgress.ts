import { onMounted, onUnmounted, ref, type Ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ScanProgress } from "../types";

const SCAN_PROGRESS_EVENT = "scan://progress";

/**
 * Listen to backend scan progress events for one task.
 * Returns a ref holding the latest progress payload; call reset() before
 * starting a new scan.
 */
export function useScanProgress(task: ScanProgress["task"]): {
  progress: Ref<ScanProgress | null>;
  reset: () => void;
} {
  const progress = ref<ScanProgress | null>(null);
  let unlisten: UnlistenFn | undefined;

  onMounted(async () => {
    unlisten = await listen<ScanProgress>(SCAN_PROGRESS_EVENT, (event) => {
      if (event.payload.task === task) progress.value = event.payload;
    });
  });

  onUnmounted(() => unlisten?.());

  return { progress, reset: () => (progress.value = null) };
}
