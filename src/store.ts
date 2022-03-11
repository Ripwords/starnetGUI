import { defineStore } from 'pinia'
import { useStorage } from '@vueuse/core'

export const mainStore = defineStore('mainStore', {
  state: () => ({
    starnetPath: useStorage('starnetPath', ''),
    outputFilename: useStorage('outputFilename', 'starless'),
    tempFile: useStorage('tempFile', "starnetTempInput"),
    autoScroll: useStorage('autoScroll', false)
  })
})