<script lang="ts" setup>
import { open } from '@tauri-apps/api/dialog'
import { platform } from '@tauri-apps/api/os'
import { dirname, resolve } from '@tauri-apps/api/path'
import { listen, emit } from '@tauri-apps/api/event'
import { copyFile } from '@tauri-apps/api/fs'
import { useLoadingBar, useMessage } from 'naive-ui'
import { mainStore } from '../store'
import { useTimeoutFn } from '@vueuse/core'

// Load Functions
const store = mainStore()
const message = useMessage()
const loading = useLoadingBar()

// Define Variables and refs
const fileInputArray = ref<string[]>([])
const outputPathRef = ref('')
const stdOut = ref('')
const stride = ref()
const stopButtonRef = ref(false)
const customStride = ref(false)
const customStrideValue = ref()
const os = await platform()
const pid = ref()
const percentage = ref(0)
const percentageRef = ref(false)
const stopTimeout = ref<any>()
let payloadLength = 0

// Finishes loading animation from fallback template
loading.finish()

// Clear the output and resets the parameters
// Removes the input file
const clear = async () => {
  emit("remove-input", [
    await resolve(`${store.starnetPath}/${store.tempFile}.tiff`)
  ])
  percentage.value = 0
  stopButtonRef.value = false
  stdOut.value = ''
  fileInputArray.value = []
  percentageRef.value = false
}

// StarNet Function
const starnet = async (
  inputPath: string, 
  counter: number, 
  outputPath?: string
) => {
  // Clear the output
  stdOut.value = ''

  // Check for StarNet path
  if (store.starnetPath == '') {
    message.error('Starnet path is not set')
    clear()
    return
  }

  // Copy Input Image to StarNet directory if Input Image is not already in Starnet directory
  if ((inputPath != `${store.starnetPath}/${store.tempFile}.tiff`) && (inputPath != `${store.starnetPath}\\${store.tempFile}.tiff`)) {
    try {
      await copyFile(inputPath, await resolve(`${store.starnetPath}/${store.tempFile}.tiff`))
    } catch {
      message.error('Image Path Error')
      clear()
      return     
    }
  }

  // Set the image output path
  if (!outputPath) {
    outputPath = await dirname(inputPath)
  }

  // Set output path
  counter > 0 ? outputPath = `${outputPath}/${store.outputFilename}_${counter}.tiff` : outputPath = `${outputPath}/${store.outputFilename}.tiff`

  // Construct Command
  const cwd = os == "windows" ? `${store.starnetPath}\\` : `${store.starnetPath}/`
  const starnetCommand = os == 'windows' ? `${store.starnetPath}\\starnet++` : `${store.starnetPath}/starnet++`

  // Emit event for rust backend to run command
  emit("starnet-command", [
    starnetCommand, 
    cwd,
    `${store.tempFile}.tiff`,
    outputPath, 
    customStride.value ? customStrideValue.value : (stride.value ? '128' : '256')
  ])

  // Show the stop buttons
  stopButtonRef.value = true

  // Set timeout to check if starnet++ crashed
  // Crash occurs when starnet path is incorrect
  const { stop } = useTimeoutFn(() => {
    if (stdOut.value == "") {
      loading.error()
      clear()
      message.error('Starnet Timed Out')
      message.error('Check Starnet Directory')
    }
  }, 10000)
  stopTimeout.value = stop
}

// Kills and abort StarNet operation
// Stops the crash check timeout
const killStarnet = async () => {
  stopTimeout.value()
  if (os == "win32") {
    emit("kill-command", ["taskkill", pid.value])
  } else {
    emit("kill-command", ["kill", pid.value])
  }
}

// Open directories or files
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
      if (fileInputArray.value.indexOf(f) == -1) {
        fileInputArray.value.push(f)
      } else {
        message.error('Image already added!')
      }
    })
  } else if (path == 'output') {
    const openDialog = await open({
      directory: true
    })
    outputPathRef.value = (openDialog == null ? '' : openDialog.toString())
  }
}

// Initialize the StarNet operation
const starnetInit = async () => {
  percentageRef.value = true
  const length = fileInputArray.value.length
  const arr = [...fileInputArray.value]
  if (arr.length == 0) {
    message.error('No image selected!')
    clear()
    return
  }
  outputPathRef.value == '' ? outputPathRef.value = await dirname(arr[length - 1]) : outputPathRef.value
  await starnet(arr[length - 1], length - 1, outputPathRef.value)
}

// Function to re-run the StarNet operation
// Resets certain parameters
// Remove the finished input file from array
const runStarnetInit = () => {
  percentage.value = 0
  percentageRef.value = false

  fileInputArray.value.pop()
  if (fileInputArray.value.length > 0) {
    starnetInit()
  } else {
    clear()
    outputPathRef.value = ''
  }
}

// Tauri Event listeners
// Listen from file drop
await listen('tauri://file-drop', (file: any) => {
  file.payload.forEach((f: any) => {
    if (fileInputArray.value.indexOf(f) == -1 && (f.endsWith('.tiff') || f.endsWith('.tif'))) {
      fileInputArray.value.push(f)
    } else if (fileInputArray.value.indexOf(f) != -1) {
      message.error('Image already added!')
    } else if (!(f.endsWith('.tiff') || f.endsWith('.tif'))) {
      message.error('Invalid File Type!')
    }
  })
})

// Kills starnet when tauri closes
await listen('tauri://close-requested', async () => {
  await killStarnet()
})

// Get PID from backend
await listen('get-pid', (data: any) => {
  pid.value = data.payload
})

// Listen for command stdout
await listen('starnet-command-stdout', (data: any) => {
  stopTimeout.value()
  if (store.autoScroll) {
    window.scrollTo(0,document.body.scrollHeight)
  }
  if (data.payload.endsWith('finished')) {
    payloadLength = data.payload.length
    if(stdOut.value.endsWith("finished")) {
      stdOut.value = stdOut.value.slice(0, stdOut.value.length - payloadLength - 1)
    }
    percentage.value = Number(data.payload.split("%")[0].trim())
  }
  stdOut.value += `\n${data.payload}`
})

// Listen for command errors
await listen('starnet-command-err', () => {
  loading.error()
  message.error('Starnet Error')
})

// Listen for command initialization
await listen('starnet-command-init', () => {
  loading.start()
  message.info('Running StarNet...')
})

// Listen for command termination
await listen('starnet-command-terminated', (data: any) => {
  if(data.payload.code == 0) {
    loading.finish()
    message.success('StarNet finished!')
    runStarnetInit()
  } else if(data.payload.code == 1 || data.payload.signal == 9) {
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
  <n-collapse :default-expanded-names="store.starnetPath == '' ? ['1'] : ['2', '4']">
    <div class="mt-4">
      <n-collapse-item title="GUI Settings" name="1">
        <div class="mx-5 mb-5">
          <n-card title="Starnet Directory">
            <n-button @click="browse('starnet')">Browse</n-button>
            <n-card v-if="store.starnetPath != ''">{{ store.starnetPath }}</n-card>
          </n-card>
        </div>
        <div class="mx-5 mb-5">
          <n-card title="Starnet Temporary file name">
            <n-input v-model:value="store.tempFile">
              <template #suffix>
                .tiff
              </template>
            </n-input>
            <template #footer>
              File will be deleted when command terminates
            </template>
          </n-card>
        </div>
        <div class="mx-5 mb-5">
          <n-card title="Auto Scroll">
            <n-checkbox v-model:checked="store.autoScroll">Auto scorll</n-checkbox>
          </n-card>
        </div>
      </n-collapse-item>
    </div>
    <n-collapse-item title="Input" name="2">
      <div class="mx-5 mb-5" v-show="store.starnetPath != ''">
        <n-card title="Input Image">
          <n-button @click="browse('input')">Browse</n-button>
          <div v-show="fileInputArray.length != 0">
            <n-card class="break-words" v-for="img in fileInputArray" :key="img">
              <div class="flex justify-between">
                <div class="max-w-[90%] relative">
                  {{ img }}
                </div>
                <button @click="fileInputArray = fileInputArray.filter(e => e !== img)">x</button>
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
          <n-card v-if="outputPathRef != ''">{{ outputPathRef }}</n-card>
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
          <n-button v-show="stopButtonRef" @click="killStarnet()">Stop</n-button>
          <n-progress v-if="percentageRef" class="absolute right-9 top-7" type="circle" :percentage="percentage" />
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