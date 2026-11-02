<script setup lang="ts">
import type { Detection } from "~/composables/useStarnet";

defineProps<{ detection: Detection }>();
defineEmits<{ locate: []; download: []; retry: [] }>();
</script>

<template>
  <div class="m-auto flex max-w-md flex-col items-center gap-4 p-6 text-center">
    <UIcon name="i-lucide-telescope" class="size-10 text-primary" />
    <template v-if="detection.kind === 'legacy'">
      <h2 class="text-lg font-semibold">This StarNet version is too old</h2>
      <p class="text-sm text-muted">
        StarNet GUI needs the current StarNet2 command-line tool. Install it, then check again.
      </p>
    </template>
    <template v-else>
      <h2 class="text-lg font-semibold">Install StarNet2 to get started</h2>
      <p class="text-sm text-muted">
        StarNet GUI runs the free StarNet2 command-line tool. Use the installer, then check again.
        If you used the ZIP, choose the folder you extracted it to.
      </p>
    </template>
    <div class="flex flex-wrap justify-center gap-2">
      <UButton icon="i-lucide-download" label="Download StarNet2" @click="$emit('download')" />
      <UButton
        icon="i-lucide-folder-search"
        label="Choose folder"
        color="neutral"
        variant="outline"
        @click="$emit('locate')"
      />
      <UButton label="Check again" color="neutral" variant="ghost" @click="$emit('retry')" />
    </div>
  </div>
</template>
