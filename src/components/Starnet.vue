<script lang="ts" setup>
import { open } from '@tauri-apps/api/dialog'
import { dirname } from '@tauri-apps/api/path'
import { listen } from '@tauri-apps/api/event'
import { Command } from '@tauri-apps/api/shell'
import { copyFile, removeFile } from '@tauri-apps/api/fs'
import { useLoadingBar, useMessage } from 'naive-ui'
import { mainStore } from '../store'

const store = mainStore()

const message = useMessage()
const loading = useLoadingBar()

const input = ref('')
const output = ref('')

const stdOut = ref('')
const stride = ref()
const mode = ref('')
const done = ref(false)

// StarNet Function
const starnet = async (type: string, inputPath: string, outputPath?: string) => {
  // Clear the output
  stdOut.value = ''

  // Check for StarNet path
  if (store.starnetPath == '') {
    message.error('Starnet path is not set')
    return
  }

  // Copy Input Image to StarNet directory
  try {
    await copyFile(inputPath, `${store.starnetPath}/input.tiff`)
  } catch(e) {
    message.error('No Image Path provided')
    return
  }

  // Set the image type (RGB/M)
  mode.value = type

  // Set the image output path
  if (!outputPath) {
    outputPath = await dirname(inputPath)
  }

  // Construct Command
  const command = new Command(
    `${store.starnetPath}\\${type}_starnet++.exe`, 
    [
      'input.tiff', 
      `${outputPath}\\${store.outputFilename}.tiff`, 
      stride.value ? '128' : '256'
    ], 
    {
      cwd: `${store.starnetPath}\\`
    }
  )

  // Get stdout and listen to events
  command.on('error', () => loading.error())
  command.on('close', () => {
    loading.finish()
    done.value = true
    message.success('StarNet finished!')
  })
  command.stdout.on('data', (line: string) => {
    line.endsWith('finished\r') ? stdOut.value += '' : stdOut.value += `${line}\n`
  })

  // Run StarNet
  try {
    loading.start()
    await command.execute()
  } catch(e) {
    loading.error()
    message.error("StarNet not found")    
    mode.value = ''
    await removeFile(`${store.starnetPath}/input.tiff`)
    return
  }
}

// Kills and abort StarNet operation
const killStarnet = async (type: string) => {
  const kill = new Command('taskkill', ['/f', '/im', `${type}_starnet++.exe`])
  kill.execute()
  stdOut.value = ''
  mode.value = ''
  await removeFile(`${store.starnetPath}/input.tiff`)
}

// Open directories/files
const browse = async (path: string) => {
  if (path == 'starnet') {
    store.starnetPath = (await open({ directory: true })).toString()
  } else {
    path == 'input' ? input.value = (await open()).toString() : output.value = (await open({ directory: true })).toString()
  }
}

// Clear the output and resets the parameters
const clear = async () => {
  stdOut.value = ''
  mode.value = ''
  await removeFile(`${store.starnetPath}/input.tiff`)
  done.value = false
}

await listen('tauri://file-drop', (file: any) => {
  input.value = file.payload[0]
})

await listen('tauri://close-requested', async () => {
  await killStarnet(mode.value)
})
</script>

<template>
  <n-collapse :default-expanded-names="['2', '4']">
    <div class="mt-4">
      <n-collapse-item title="StarNet Directory" name="1">
        <div class="mx-5 mb-5">
          <n-card title="StarNet Directory">
            <n-button @click="browse('starnet')">Browse</n-button>
            <n-card v-if="store.starnetPath != ''">{{ store.starnetPath }}</n-card>
          </n-card>
        </div>
      </n-collapse-item>
    </div>
    <n-collapse-item title="Input" name="2">
      <div class="mx-5 mb-5" v-show="store.starnetPath != ''">
        <n-card title="Input Image">
          <n-button @click="browse('input')">Browse</n-button>
          <n-card v-show="input != ''">{{ input }}</n-card>
        </n-card>
      </div>
    </n-collapse-item>
    <n-collapse-item title="Output" name="3">
      <div class="mx-5 mb-5" v-show="store.starnetPath != ''">
        <n-card title="Output">
          <n-button @click="browse('output')">Browse</n-button>
          <n-card v-if="output != ''">{{ output }}</n-card>
          <n-card v-else>Defaults to input directory if not provided</n-card>
          <br>
          <br>
          <n-input placeholder="starless" v-model:value="store.outputFilename" spellcheck="false">
            <template #suffix>
              .tiff
            </template>
          </n-input>
        </n-card>
      </div>
    </n-collapse-item>
    <n-collapse-item title="StarNet++" name="4">
      <div class="mx-5 mb-5" v-show="store.starnetPath != ''">
        <n-card title="StarNet++">
          <n-button @click="starnet('rgb', input, output)">StarNet RGB</n-button>
          <n-button @click="starnet('mono', input, output)">StarNet Mono</n-button>
          <n-button v-show="mode != '' && !done" @click="killStarnet(mode)">Stop</n-button>
          <n-button v-show="done" @click="clear">Done</n-button>
          <div class="my-3">
            <n-checkbox v-model:checked="stride">Finer Tiles</n-checkbox>
          </div>
          <span class="whitespace-pre-line">{{ stdOut }}</span>
        </n-card>
      </div>
    </n-collapse-item>
  </n-collapse>
</template>

<style>
::-webkit-scrollbar {
  width: 0;
  background: transparent;
}

html {
  overflow: auto;
  overflow-x: hidden;
}
</style>