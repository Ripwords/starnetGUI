<script setup lang="ts">
defineProps<{ percent: number; label: string }>();
</script>

<template>
  <div
    role="progressbar"
    :aria-label="label"
    aria-valuemin="0"
    aria-valuemax="100"
    :aria-valuenow="percent"
    class="h-1.5 overflow-hidden rounded-full bg-accented"
  >
    <div
      class="h-full rounded-full bg-primary transition-[width] duration-200 ease-out motion-reduce:transition-none"
      :style="{ width: `${percent}%` }"
    />
  </div>
</template>
