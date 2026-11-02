export default defineNuxtConfig({
  modules: ["@nuxt/ui"],
  ssr: false,
  devtools: { enabled: false },
  css: ["~/assets/css/main.css"],
  compatibilityDate: "2026-06-30",
  icon: { clientBundle: { scan: true, sizeLimitKb: 512 } },
  ui: { fonts: false },
  colorMode: { preference: "dark", fallback: "dark" },
  vite: {
    clearScreen: false,
    envPrefix: ["VITE_", "TAURI_"],
    server: { strictPort: true },

    optimizeDeps: {
      include: [
        '@tauri-apps/api/core',
        '@tauri-apps/api/webview',
        '@tauri-apps/plugin-dialog',
        '@tauri-apps/plugin-opener',
        '@tauri-apps/plugin-process',
        '@tauri-apps/plugin-updater',
        'canvas-confetti',
      ]
    }
  },
  ignore: ["**/src-tauri/**"],
});
