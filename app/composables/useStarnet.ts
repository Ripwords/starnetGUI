import { Channel, invoke } from "@tauri-apps/api/core";
import { useLocalStorage } from "@vueuse/core";

export type Detection =
  | { kind: "found"; path: string; version: string }
  | { kind: "legacy"; path: string }
  | { kind: "notFound" };

export type RunEvent = { type: "progress"; percent: number } | { type: "log"; line: string };

export type RunOutcome =
  | { kind: "done"; outputs: string[]; warnings: string[] }
  | { kind: "cancelled" }
  | { kind: "failed"; message: string };

export type LogLine = { name: string; text: string };

export type QueueItem = { path: string; name: string } & (
  | { status: "queued" }
  | { status: "running"; percent: number | null }
  | { status: "done"; outputs: string[]; warnings: string[] }
  | { status: "cancelled" }
  | { status: "failed"; message: string }
);

const SUPPORTED = /\.(tiff?|png|fits?|fts)(\.gz)?$/i;
const COMPRESSED_NON_FITS = /\.(tiff?|png)\.gz$/i;

export const isSupportedImage = (path: string) =>
  SUPPORTED.test(path) && !COMPRESSED_NON_FITS.test(path);

const MAX_LOG_LINES = 5000;
const PROGRESS_LOG_STEP = 10;

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

export function useStarnet() {
  const customPath = useLocalStorage<string | null>("starnet-folder", null);
  const outputDir = useLocalStorage<string | null>("output-folder", null);
  const options = useLocalStorage(
    "run-options",
    {
      upsample: false,
      linear: false,
      shadowsClipping: null as number | null,
      targetBackground: null as number | null,
      highlightProtection: true,
      mask: false,
      unscreen: false,
      stride: null as number | null,
      concurrency: 2,
    },
    { mergeDefaults: true },
  );

  const detection = ref<Detection | null>(null);
  const queue = ref<QueueItem[]>([]);
  const log = ref<LogLine[]>([]);
  const running = ref(false);
  const cancelRequested = ref(false);

  async function detect() {
    detection.value = await invoke<Detection>("detect_starnet", { custom: customPath.value });
  }

  function add(paths: string[]) {
    const known = new Set(queue.value.map((item) => item.path));
    const fresh = [...new Set(paths)].filter((path) => !known.has(path));
    const accepted = fresh.filter(isSupportedImage);
    queue.value.push(
      ...accepted.map((path): QueueItem => ({ path, name: fileName(path), status: "queued" })),
    );
    return { added: accepted.length, rejected: fresh.length - accepted.length };
  }

  function remove(path: string) {
    queue.value = queue.value.filter((item) => item.path !== path);
  }

  function clearFinished() {
    queue.value = queue.value.filter((item) => item.status === "queued");
  }

  function requeueFinished() {
    queue.value = queue.value.map((item) =>
      item.status === "queued" || item.status === "running"
        ? item
        : { path: item.path, name: item.name, status: "queued" },
    );
  }

  function replace(path: string, next: QueueItem) {
    const index = queue.value.findIndex((item) => item.path === path);
    if (index !== -1) queue.value[index] = next;
  }

  function appendLog(name: string, text: string) {
    log.value.push({ name, text });
    if (log.value.length > MAX_LOG_LINES) log.value.splice(0, log.value.length - MAX_LOG_LINES);
  }

  async function runOne(binary: string, item: QueueItem) {
    const base = { path: item.path, name: item.name };
    replace(item.path, { ...base, status: "running", percent: null });

    let loggedStep = -1;
    const events = new Channel<RunEvent>();
    events.onmessage = (event) => {
      if (event.type === "log") {
        appendLog(item.name, event.line);
        return;
      }
      replace(item.path, { ...base, status: "running", percent: event.percent });
      const step = Math.floor(event.percent / PROGRESS_LOG_STEP);
      if (step > loggedStep) {
        loggedStep = step;
        appendLog(item.name, `${step * PROGRESS_LOG_STEP}%`);
      }
    };

    const { linear, shadowsClipping, targetBackground, concurrency: _, ...rest } = options.value;
    const outcome = await invoke<RunOutcome>("run_starnet", {
      request: {
        ...rest,
        binary,
        input: item.path,
        outputDir: outputDir.value,
        linear: linear ? { shadowsClipping, targetBackground } : null,
      },
      events,
    }).catch((error): RunOutcome => ({ kind: "failed", message: String(error) }));

    if (outcome.kind === "done") {
      replace(item.path, { ...base, status: "done", outputs: outcome.outputs, warnings: outcome.warnings });
    } else if (outcome.kind === "cancelled") {
      replace(item.path, { ...base, status: "cancelled" });
    } else {
      replace(item.path, { ...base, status: "failed", message: outcome.message });
    }
  }

  async function start() {
    if (running.value || detection.value?.kind !== "found") return;
    const binary = detection.value.path;
    running.value = true;
    cancelRequested.value = false;
    log.value = [];
    await invoke("begin_batch");

    async function worker() {
      let next: QueueItem | undefined;
      while (
        !cancelRequested.value &&
        (next = queue.value.find((item) => item.status === "queued"))
      ) {
        await runOne(binary, next);
      }
    }
    await Promise.all(Array.from({ length: options.value.concurrency }, worker));
    running.value = false;
  }

  async function stop() {
    cancelRequested.value = true;
    await invoke("cancel_starnet");
  }

  return {
    customPath,
    outputDir,
    options,
    detection,
    queue,
    log,
    running,
    cancelRequested,
    detect,
    add,
    remove,
    clearFinished,
    requeueFinished,
    start,
    stop,
  };
}
