import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

export type UpdaterState =
  | { phase: "idle" }
  | { phase: "available"; update: Update }
  | { phase: "downloading"; update: Update; percent: number }
  | { phase: "failed"; message: string };

export function useUpdater() {
  const state = shallowRef<UpdaterState>({ phase: "idle" });

  async function checkSilently() {
    try {
      const update = await check();
      if (update) state.value = { phase: "available", update };
    } catch (error) {
      console.warn("Update check failed", error);
    }
  }

  async function install() {
    if (state.value.phase !== "available") return;
    const { update } = state.value;
    let downloaded = 0;
    let total = 0;
    state.value = { phase: "downloading", update, percent: 0 };
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          const percent = total > 0 ? Math.round((downloaded / total) * 100) : 0;
          state.value = { phase: "downloading", update, percent };
        }
      });
      await relaunch();
    } catch (error) {
      state.value = { phase: "failed", message: String(error) };
    }
  }

  function dismiss() {
    state.value = { phase: "idle" };
  }

  return { state, checkSilently, install, dismiss };
}
