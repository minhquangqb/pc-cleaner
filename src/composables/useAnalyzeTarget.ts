import { ref } from "vue";

/**
 * One-shot handoff of a path from DashboardView to TreeView: the dashboard
 * requests analysis of a disk, App switches tabs, TreeView consumes the
 * target once when (re)activated and starts scanning it.
 */
const target = ref<string | null>(null);

export function requestAnalyze(path: string) {
  target.value = path;
}

export function consumeAnalyzeTarget(): string | null {
  const path = target.value;
  target.value = null;
  return path;
}
