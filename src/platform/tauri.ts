import { convertFileSrc, invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { message, open, save } from "@tauri-apps/plugin-dialog"
import { openPath, openUrl } from "@tauri-apps/plugin-opener"
import { load } from "@tauri-apps/plugin-store"
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart"
import type { Platform } from "./types"

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
}

export const tauriPlatform: Platform = {
  kind: "desktop",
  invoke: (command, args) => invoke(command, args),
  listen: (event, handler) => listen(event, handler),
  convertFileSrc: (path) => convertFileSrc(path),
  createHttpFetch: () => import("@tauri-apps/plugin-http").then((m) => m.fetch),
  loadStore: (name) => load(name, { autoSave: true, defaults: {} }),
  openDialog: (options) => open(options),
  saveDialog: (options) => save(options),
  messageDialog: async (text, options) => {
    await message(text, options)
  },
  openUrl: (url) => openUrl(url),
  openPath: (path) => openPath(path),
  isAutostartEnabled: () => isAutostartEnabled(),
  setAutostart: (enabled) => (enabled ? enableAutostart() : disableAutostart()),
  setWindowTheme: (theme, background) => {
    if (!isTauriRuntime()) return
    const win = getCurrentWindow()
    void win.setTheme(theme).catch((err) => {
      console.warn("[theme] failed to sync native window theme:", err)
    })
    void win.setBackgroundColor(background).catch((err) => {
      console.warn("[theme] failed to sync native window background:", err)
    })
  },
}
