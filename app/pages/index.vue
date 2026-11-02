<script setup lang="ts">
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import confetti from "canvas-confetti";

const DOWNLOAD_URL = "https://starnetastro.com/cli-tools/starnet/";
const IMAGE_EXTENSIONS = ["tif", "tiff", "png", "fit", "fits", "fts", "gz"];
const CONCURRENCY_CHOICES = [1, 2, 3, 4];

const toast = useToast();
const starnet = useStarnet();
const { detection, queue, options, outputDir, running, log, cancelRequested } = starnet;

const dragging = ref(false);
const logOpen = ref(false);
const advancedOpen = ref(false);

const switchUi = { root: "flex-row-reverse justify-between gap-4", wrapper: "ms-0" };

const queuedCount = computed(() => queue.value.filter((i) => i.status === "queued").length);
const finishedCount = computed(
  () => queue.value.filter((i) => i.status !== "queued" && i.status !== "running").length,
);
const total = computed(() => queue.value.length);
const queueSummary = computed(() =>
  running.value
    ? `${finishedCount.value} of ${total.value} done`
    : total.value === 1
      ? "1 image"
      : `${total.value} images`,
);
const outputFolderName = computed(() => outputDir.value?.split(/[\\/]/).filter(Boolean).pop());
const advancedChanges = computed(() => {
  const { highlightProtection, stride, linear, shadowsClipping, targetBackground } = options.value;
  return [
    !highlightProtection,
    stride !== null,
    linear && shadowsClipping !== null,
    linear && targetBackground !== null,
  ].filter(Boolean).length;
});

async function run() {
  await starnet.start();
  const allSucceeded = total.value > 0 && queue.value.every((item) => item.status === "done");
  if (!allSucceeded) return;
  confetti({
    particleCount: 70,
    spread: 70,
    startVelocity: 28,
    ticks: 110,
    scalar: 0.8,
    origin: { x: 0.33, y: 0.7 },
    colors: ["#ffffff", "#d4d4d4", "#a3a3a3"],
    disableForReducedMotion: true,
  });
}

function runAgain() {
  starnet.requeueFinished();
  run();
}

function addFiles(paths: string[]) {
  const { rejected } = starnet.add(paths);
  if (rejected > 0) {
    toast.add({
      title: rejected === 1 ? "One file was skipped" : `${rejected} files were skipped`,
      description: "StarNet2 reads TIFF, PNG and FITS images.",
      color: "warning",
    });
  }
}

async function browseImages() {
  const picked = await open({
    multiple: true,
    filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }],
  });
  if (picked) addFiles(picked);
}

async function chooseOutput() {
  const picked = await open({ directory: true });
  if (picked) outputDir.value = picked;
}

async function locateStarnet() {
  const picked = await open({ directory: true });
  if (!picked) return;
  starnet.customPath.value = picked;
  await starnet.detect();
  if (detection.value?.kind === "notFound") {
    toast.add({
      title: "StarNet2 was not found in that folder",
      description: "Choose the folder that holds starnet2, or its parent.",
      color: "error",
    });
  }
}

let unlisten: (() => void) | undefined;
onMounted(async () => {
  await starnet.detect();
  unlisten = await getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type === "enter" || payload.type === "over") dragging.value = true;
    else dragging.value = false;
    if (payload.type === "drop") addFiles(payload.paths);
  });
});
onBeforeUnmount(() => unlisten?.());
</script>

<template>
  <div class="flex h-dvh flex-col">
    <h1 class="sr-only">StarNet GUI</h1>
    <UpdateBanner class="m-3 mb-0 shrink-0" />

    <StarnetSetup
      v-if="detection && detection.kind !== 'found'"
      :detection="detection"
      @locate="locateStarnet"
      @download="openUrl(DOWNLOAD_URL)"
      @retry="starnet.detect"
    />

    <div v-else-if="detection" class="flex min-h-0 flex-1">
      <main class="relative flex min-w-0 flex-1 flex-col">
        <button
          v-if="total === 0"
          type="button"
          class="m-4 flex flex-1 flex-col items-center justify-center gap-1 rounded-lg border border-dashed border-accented transition-colors hover:bg-muted focus-visible:outline-2 focus-visible:outline-primary"
          @click="browseImages"
        >
          <UIcon name="i-lucide-image-plus" class="mb-2 size-7 text-muted" />
          <span class="font-medium">Drop images here</span>
          <span class="text-sm text-muted">or click to browse. TIFF, PNG or FITS.</span>
        </button>

        <template v-else>
          <div class="flex items-center gap-2 py-2 ps-4 pe-2">
            <span class="me-auto text-sm text-muted tabular-nums">{{ queueSummary }}</span>
            <UButton
              v-if="log.length > 0"
              icon="i-lucide-scroll-text"
              label="Log"
              color="neutral"
              :variant="logOpen ? 'soft' : 'ghost'"
              :aria-pressed="logOpen"
              @click="logOpen = !logOpen"
            />
            <UButton
              v-if="finishedCount > 0 && !running"
              label="Clear finished"
              color="neutral"
              variant="ghost"
              @click="starnet.clearFinished"
            />
            <UButton
              icon="i-lucide-plus"
              label="Add"
              color="neutral"
              variant="ghost"
              @click="browseImages"
            />
          </div>
          <ul class="min-h-0 flex-1 divide-y divide-default overflow-y-auto border-t border-default">
            <QueueRow
              v-for="item in queue"
              :key="item.path"
              :item="item"
              @remove="starnet.remove(item.path)"
            />
          </ul>
          <LogPanel v-if="logOpen && log.length > 0" :lines="log" />
        </template>

        <div
          v-if="dragging"
          class="pointer-events-none absolute inset-2 flex items-center justify-center rounded-lg border-2 border-primary bg-primary/10 font-medium"
        >
          Drop to add
        </div>
      </main>

      <aside
        aria-label="Options"
        class="flex w-72 shrink-0 flex-col border-s border-default bg-muted"
      >
        <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-4 [scrollbar-width:none]">
          <USwitch
            v-model="options.linear"
            :ui="switchUi"
            label="Linear image"
            description="For unstretched data"
          />
          <USwitch
            v-model="options.upsample"
            :ui="switchUi"
            label="Upsample"
            description="For tight stars. Slower."
          />
          <USwitch
            v-model="options.mask"
            :ui="switchUi"
            label="Save star mask"
            description="Adds a _starmask file"
          />
          <USwitch
            v-model="options.unscreen"
            :ui="switchUi"
            label="Save star layer"
            description="Adds a _stars file"
          />

          <div class="flex items-center justify-between gap-2 border-t border-default pt-4">
            <div class="min-w-0 text-sm">
              <p class="font-medium">Images at once</p>
              <p class="text-muted">Uses more memory</p>
            </div>
            <UFieldGroup>
              <UButton
                v-for="choice in CONCURRENCY_CHOICES"
                :key="choice"
                :label="String(choice)"
                :aria-pressed="options.concurrency === choice"
                :disabled="running"
                color="neutral"
                :variant="options.concurrency === choice ? 'solid' : 'outline'"
                square
                class="w-7 justify-center px-0"
                @click="options.concurrency = choice"
              />
            </UFieldGroup>
          </div>

          <div class="flex items-center gap-2">
            <div class="min-w-0 flex-1 text-sm" :title="outputDir ?? undefined">
              <p class="font-medium">Save to</p>
              <p class="truncate text-muted">{{ outputFolderName ?? "Next to each image" }}</p>
            </div>
            <UButton
              v-if="outputDir"
              icon="i-lucide-x"
              color="neutral"
              variant="ghost"
              square
              aria-label="Save next to each image"
              @click="outputDir = null"
            />
            <UButton label="Change" color="neutral" variant="outline" @click="chooseOutput" />
          </div>

          <UCollapsible v-model:open="advancedOpen" class="border-t border-default pt-2">
            <UButton
              color="neutral"
              variant="ghost"
              class="group -mx-2.5 w-[calc(100%+1.25rem)]"
            >
              <span>Advanced</span>
              <span v-if="advancedChanges > 0" class="font-normal text-muted">
                {{ advancedChanges }} changed
              </span>
              <UIcon
                name="i-lucide-chevron-down"
                class="ms-auto size-5 transition-transform group-data-[state=open]:rotate-180"
              />
            </UButton>
            <template #content>
              <div class="flex flex-col gap-4 pt-2 pb-1">
                <USwitch
                  v-model="options.highlightProtection"
                  :ui="switchUi"
                  label="Protect highlights"
                  description="Turn off only to compare"
                />
                <UFormField label="Stride" description="Even, from 2 to 512">
                  <UInputNumber
                    v-model="options.stride"
                    :min="2"
                    :max="512"
                    :step="2"
                    placeholder="Default"
                    class="w-full"
                  />
                </UFormField>
                <template v-if="options.linear">
                  <UFormField label="Shadows clipping" description="Negative. Default -2.8.">
                    <UInputNumber
                      v-model="options.shadowsClipping"
                      :min="-10"
                      :max="-0.1"
                      :step="0.1"
                      placeholder="Default"
                      class="w-full"
                    />
                  </UFormField>
                  <UFormField label="Target background" description="0 to 1. Default 0.25.">
                    <UInputNumber
                      v-model="options.targetBackground"
                      :min="0.01"
                      :max="0.99"
                      :step="0.01"
                      placeholder="Default"
                      class="w-full"
                    />
                  </UFormField>
                </template>
                <div v-if="detection.kind === 'found'" class="flex flex-col gap-2 text-sm">
                  <p class="font-medium">StarNet2 {{ detection.version }}</p>
                  <p class="break-all text-muted select-text">{{ detection.path }}</p>
                  <div class="flex flex-wrap gap-2">
                    <UButton
                      label="Choose folder"
                      color="neutral"
                      variant="outline"
                      @click="locateStarnet"
                    />
                    <UButton
                      v-if="starnet.customPath.value"
                      label="Use automatic"
                      color="neutral"
                      variant="ghost"
                      @click="
                        starnet.customPath.value = null;
                        starnet.detect();
                      "
                    />
                  </div>
                </div>
              </div>
            </template>
          </UCollapsible>
        </div>

        <div class="border-t border-default p-4">
          <UButton
            v-if="running"
            block
            icon="i-lucide-square"
            :label="cancelRequested ? 'Stopping' : 'Stop'"
            color="neutral"
            variant="outline"
            :disabled="cancelRequested"
            @click="starnet.stop"
          />
          <UButton
            v-else-if="queuedCount === 0 && finishedCount > 0"
            block
            icon="i-lucide-rotate-ccw"
            label="Run again"
            color="neutral"
            variant="outline"
            @click="runAgain"
          />
          <UButton
            v-else
            block
            icon="i-lucide-play"
            label="Remove stars"
            :disabled="queuedCount === 0"
            @click="run"
          />
        </div>
      </aside>
    </div>
  </div>
</template>
