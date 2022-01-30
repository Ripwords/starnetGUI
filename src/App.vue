<script lang="ts" setup>
import { darkTheme } from 'naive-ui'
</script>

<template>
  <n-config-provider :theme="darkTheme">
    <n-global-style />
    <n-loading-bar-provider>
      <n-message-provider>
        <Suspense>
          <template #default>
            <Starnet />
          </template>
          <template #fallback>
            <div>Loading...</div>
          </template>
        </Suspense>
      </n-message-provider>
    </n-loading-bar-provider>
  </n-config-provider>
</template>