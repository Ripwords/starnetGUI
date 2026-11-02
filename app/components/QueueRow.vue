<script setup lang="ts">
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { QueueItem } from "~/composables/useStarnet";

defineProps<{ item: QueueItem }>();
defineEmits<{ remove: [] }>();
</script>

<template>
  <li class="flex flex-col gap-1.5 py-2 ps-4 pe-2">
    <div class="flex min-h-8 items-center gap-3">
      <UIcon
        v-if="item.status === 'running'"
        name="i-lucide-loader-circle"
        class="size-5 shrink-0 animate-spin text-primary"
      />
      <UIcon
        v-else-if="item.status === 'done'"
        name="i-lucide-circle-check"
        class="size-5 shrink-0 text-success"
      />
      <UIcon
        v-else-if="item.status === 'failed'"
        name="i-lucide-circle-alert"
        class="size-5 shrink-0 text-error"
      />
      <UIcon
        v-else-if="item.status === 'cancelled'"
        name="i-lucide-circle-slash"
        class="size-5 shrink-0 text-muted"
      />
      <UIcon v-else name="i-lucide-image" class="size-5 shrink-0 text-muted" />

      <span class="min-w-0 flex-1 truncate text-sm" :title="item.path">{{ item.name }}</span>

      <UButton
        v-if="item.status === 'done' && item.outputs[0]"
        icon="i-lucide-folder-open"
        label="Show"
        color="neutral"
        variant="ghost"
        @click="revealItemInDir(item.outputs[0]!)"
      />
      <span v-else-if="item.status === 'cancelled'" class="text-sm text-muted">Stopped</span>
      <span v-else-if="item.status === 'running'" class="pe-2 text-sm text-muted tabular-nums">
        {{ item.percent === null ? "Starting" : `${Math.round(item.percent)}%` }}
      </span>
      <UButton
        v-if="item.status !== 'running'"
        icon="i-lucide-x"
        color="neutral"
        variant="ghost"
        square
        aria-label="Remove from list"
        @click="$emit('remove')"
      />
    </div>
    <ProgressBar
      v-if="item.status === 'running'"
      :percent="item.percent ?? 0"
      :label="`Processing ${item.name}`"
      class="ms-8 me-2"
    />
    <p v-if="item.status === 'failed'" class="ms-8 me-2 pb-1 text-sm break-words text-error select-text">
      {{ item.message }}
    </p>
    <template v-if="item.status === 'done'">
      <p
        v-for="warning in item.warnings"
        :key="warning"
        class="ms-8 me-2 pb-1 text-sm break-words text-warning select-text"
      >
        {{ warning }}
      </p>
    </template>
  </li>
</template>
