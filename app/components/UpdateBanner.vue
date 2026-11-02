<script setup lang="ts">
const { state, checkSilently, install, dismiss } = useUpdater();
onMounted(checkSilently);
</script>

<template>
  <UAlert
    v-if="state.phase === 'available'"
    icon="i-lucide-arrow-up-circle"
    color="primary"
    variant="subtle"
    :title="`Version ${state.update.version} is available`"
    :actions="[
      { label: 'Install and restart', color: 'primary', onClick: install },
      { label: 'Later', color: 'neutral', variant: 'ghost', onClick: dismiss },
    ]"
  />
  <div v-else-if="state.phase === 'downloading'" class="flex flex-col gap-2 text-sm">
    <span>Downloading version {{ state.update.version }}</span>
    <ProgressBar :percent="state.percent" :label="`Downloading version ${state.update.version}`" />
  </div>
  <UAlert
    v-else-if="state.phase === 'failed'"
    icon="i-lucide-triangle-alert"
    color="error"
    variant="subtle"
    title="The update could not be installed"
    :description="state.message"
    :actions="[{ label: 'Dismiss', color: 'neutral', variant: 'ghost', onClick: dismiss }]"
  />
</template>
