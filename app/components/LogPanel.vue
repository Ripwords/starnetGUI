<script setup lang="ts">
import type { LogLine } from "~/composables/useStarnet";

const props = defineProps<{ lines: LogLine[] }>();

const FOLLOW_THRESHOLD_PX = 24;

const scroller = ref<HTMLElement>();
const following = ref(true);

function updateFollowing() {
  const el = scroller.value;
  if (!el) return;
  following.value = el.scrollHeight - el.scrollTop - el.clientHeight < FOLLOW_THRESHOLD_PX;
}

function scrollToEnd() {
  const el = scroller.value;
  if (el && following.value) el.scrollTop = el.scrollHeight;
}

watch(() => props.lines.length, scrollToEnd, { flush: "post" });
onMounted(scrollToEnd);
</script>

<template>
  <div
    ref="scroller"
    role="log"
    aria-label="StarNet2 log"
    class="h-48 shrink-0 overflow-y-auto border-t border-default bg-muted px-4 py-2 font-mono text-sm select-text"
    @scroll.passive="updateFollowing"
  >
    <p v-for="(line, index) in lines" :key="index" class="break-words">
      <span class="text-muted">{{ line.name }}</span> {{ line.text }}
    </p>
  </div>
</template>
