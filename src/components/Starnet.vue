<script lang="ts" setup>
import { open } from '@tauri-apps/api/dialog'
import { platform } from '@tauri-apps/api/os'
import { dirname } from '@tauri-apps/api/path'
import { listen, emit } from '@tauri-apps/api/event'
import { copyFile } from '@tauri-apps/api/fs'
import { useLoadingBar, useMessage } from 'naive-ui'
import { mainStore } from '../store'

const store = mainStore()
const message = useMessage()
const loading = useLoadingBar()
const input = ref<string[]>([])
const output = ref('')
const stdOut = ref('')
const stride = ref()
const mode = ref('')
const done = ref(false)
const customStride = ref(false)
const customStrideValue = ref()
const os = await platform()
const pid = ref()
let payloadLength = 0

// StarNet Function
const starnet = async (
  inputPath: string, 
  counter: number, 
  outputPath?: string
) => {
  // Clear the output
  stdOut.value = ''
  done.value = false

  // Check for StarNet path
  if (store.starnetPath == '') {
    message.error('Starnet path is not set')
    return
  }

  // Copy Input Image to StarNet directory
  try {
    if (!inputPath.endsWith('input.tiff')) {
      await copyFile(inputPath, `${store.starnetPath}/input.tiff`)
    }
  } catch(e: any) {
    message.error('Image Path Error')
    return
  }

  mode.value = "Running"

  // Set the image output path
  if (!outputPath) {
    outputPath = await dirname(inputPath)
  }

  // Check for Platform and set output path
  if (os == "windows") {
    counter > 1 ? outputPath = `${outputPath}\\${store.outputFilename}_${counter + 1}.tiff` : outputPath = `${outputPath}\\${store.outputFilename}.tiff`
  } else {
    counter > 1 ? outputPath = `${outputPath}/${store.outputFilename}_${counter + 1}.tiff` : outputPath = `${outputPath}/${store.outputFilename}.tiff`
  }

  // Construct Command
  const cwd = os == "windows" ? `${store.starnetPath}\\` : `${store.starnetPath}/`
  const starnetCommand = os == 'windows' ? `${store.starnetPath}\\starnet++` : `${store.starnetPath}/starnet++`

  emit("starnet-command", [
    starnetCommand, 
    cwd,
    outputPath, 
    customStride.value ? customStrideValue.value : (stride.value ? '128' : '256')
  ])
}

// Kills and abort StarNet operation
const killStarnet = async () => {
  if (os == "win32") {
    emit("kill-command", ["taskkill", pid.value])
  } else {
    emit("kill-command", ["killall", pid.value])
  }
}

// Open directories/files
const browse = async (path: string) => {
  if (path == 'starnet') {
    store.starnetPath = (await open({ directory: true })).toString()
  } else if (path == 'input'){
    const openDialog = await open({
      multiple: true,
      filters: [
        { name: 'TIFF', extensions: ['tiff', 'tif'] }
      ]
    })
    const file = (openDialog == null ? [] : [...openDialog])
    file.forEach((f: string) => {
      if (input.value.indexOf(f) == -1) {
        input.value.push(f)
      } else {
        message.error('Image already added!')
      }
    })
  } else if (path == 'output') {
    const openDialog = await open({
      directory: true
    })
    output.value = (openDialog == null ? '' : openDialog.toString())
  }
}

// Clear the output and resets the parameters
const clear = async () => {
  emit("remove-input")
  mode.value = ''
  stdOut.value = ''
  done.value = false
  input.value = []
}

// Initialize the StarNet operation
const starnetInit = async () => {
  const length = input.value.length
  const arr = [...input.value]
  if (arr.length == 0) {
    message.error('No image selected!')
    return
  }

  output.value == '' ? output.value = await dirname(arr[length - 1]) : output.value
  await starnet(arr[length - 1], length - 1, output.value)
  output.value = ''
}

const runStarnetInit = () => {
  input.value.shift()
  if (input.value.length > 0) {
    starnetInit()
  } else {
    clear()
  }
}

// Tauri Event listeners
await listen('tauri://file-drop', (file: any) => {
  file.payload.forEach((f: any) => {
    if (input.value.indexOf(f) == -1 && (f.endsWith('.tiff') || f.endsWith('.tif'))) {
      input.value.push(f)
    } else if (input.value.indexOf(f) != -1) {
      message.error('Image already added!')
    } else if (!(f.endsWith('.tiff') || f.endsWith('.tif'))) {
      message.error('Invalid File Type!')
    }
  })
})

await listen('tauri://close-requested', async () => {
  await killStarnet()
})

await listen('get-pid', (data: any) => {
  pid.value = data.payload
})

await listen('starnet-command-stdout', (data: any) => {
  if(data.payload.endsWith('finished')) {
    payloadLength = data.payload.length
    if(stdOut.value.endsWith("finished")) {
      stdOut.value = stdOut.value.slice(0, stdOut.value.length - payloadLength - 1)
    }
  }
  stdOut.value += `\n${data.payload}`
})

await listen('starnet-command-err', () => {
  loading.error()
  message.error('Starnet Error')
})

await listen('starnet-command-init', () => {
  loading.start()
  message.info('Running StarNet...')
})

await listen('starnet-command-terminated', (data: any) => {
  if(data.payload.code == 0) {
    loading.finish()
    message.success('StarNet finished!')
    done.value = true
    runStarnetInit()
  } else if(data.payload.code == 1) {
    loading.error()
    message.error('Starnet Aborted!')
    runStarnetInit()
  } else {
    loading.error()
    message.error('Starnet Error')
    clear()
  }
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
          <div v-show="input.length != 0">
            <n-card class="break-words" v-for="img in input" :key="img">
              <div class="flex justify-between">
                <div class="max-w-[90%] relative">
                  {{ img }}
                </div>
                <button @click="input = input.filter(e => e !== img)">x</button>
              </div>
            </n-card>
          </div>
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
          <n-button @click="starnetInit()">StarNet++</n-button>
          <n-button v-show="mode != '' && !done" @click="killStarnet()">Stop</n-button>
          <n-button v-show="done" @click="clear">Done</n-button>
          <div class="my-3">
            <n-checkbox v-model:checked="customStride">Custom Stride</n-checkbox>
            <n-checkbox v-if="!customStride" v-model:checked="stride">Finer Tiles</n-checkbox>
            <div class="mt-3">
              <n-input v-if="customStride" autosize class="w-[150px]" v-model:value="customStrideValue"></n-input>
            </div>
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