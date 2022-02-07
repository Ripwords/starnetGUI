<script lang="ts" setup>
import { open } from '@tauri-apps/api/dialog'
import { platform } from '@tauri-apps/api/os'
import { dirname } from '@tauri-apps/api/path'
import { listen } from '@tauri-apps/api/event'
import { Command } from '@tauri-apps/api/shell'
import { copyFile, removeFile } from '@tauri-apps/api/fs'
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
  const command = new Command(
    starnetCommand, 
    [
      'input.tiff', 
      outputPath, 
      customStride.value ? customStrideValue.value : (stride.value ? '128' : '256')
    ], 
    {
      cwd: cwd
    }
  )
      
  // Get stdout and listen to events
  command.on('error', () => {
    loading.error()
    message.error('Starnet Error')
    return
  })
  command.on('close', () => {
    if (mode.value != '') {
      if (stdOut.value.endsWith('errors!\n')) {
        loading.error()
        message.error('Starnet Error')
        return
      }
      loading.finish()
      done.value = true
      message.success('StarNet finished!')
    } else {
      loading.error()
      message.error('StarNet Aborted!')
    }
  })
  command.stdout.on('data', (line: string) => {
    (line.endsWith('finished\r') || line.endsWith('finished')) ? stdOut.value += '' : stdOut.value += `${line}\n`
  })

  // Run StarNet
  try {
    loading.start()
    message.info('Running StarNet...')
    await command.execute()
  } catch(e) {
    loading.error()
    message.error("StarNet not found!")    
    await removeFile(`${store.starnetPath}/input.tiff`)
    return
  }
}

// Kills and abort StarNet operation
const killStarnet = async () => {
  await clear()
  let kill
  if (os == "windows") {
    kill = new Command('taskkill', ['/f', '/im', `starnet++.exe`])
  } else {
    kill = new Command('killall', [`starnet++`])
  }
  kill.execute()
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
  mode.value = ''
  stdOut.value = ''
  await removeFile(`${store.starnetPath}/input.tiff`)
  done.value = false
}

// Initialize the StarNet operation
const starnetInit = async () => {
  const length = input.value.length
  const arr = [...input.value]
  if (arr.length == 0) {
    message.error('No image selected!')
    return
  }
  for (let i = 0; i < length; i++) {
    output.value == '' ? output.value = await dirname(arr[i]) : output.value
    await starnet(arr[i], i, output.value)
    input.value.shift()
  }
  output.value = ''
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