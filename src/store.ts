import { defineStore } from 'pinia'
import { useStorage } from '@vueuse/core'

export const mainStore = defineStore('mainStore', {
  state: () => ({
    starnetPath: useStorage('starnetPath', ''),
    outputFilename: useStorage('outputFilename', 'starless'),
  })
})