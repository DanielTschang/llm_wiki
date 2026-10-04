/**
 * The shell-facing surface the frontend is allowed to use. Everything
 * that needs the desktop runtime (Tauri IPC, native dialogs, the asset
 * protocol, plugin-store, …) goes through this interface so the same
 * React build can run inside the Tauri webview or in a plain browser
 * talking to the self-hosted server. See plans/web-server-mode.md.
 */

export type PlatformKind = "desktop" | "web"

export type UnlistenFn = () => void

export interface PlatformEvent<T> {
  event: string
  payload: T
}

export type EventHandler<T> = (event: PlatformEvent<T>) => void

export interface DialogFilter {
  name: string
  extensions: string[]
}

export interface OpenDialogOptions {
  title?: string
  defaultPath?: string
  directory?: boolean
  multiple?: boolean
  createDirectories?: boolean
  filters?: DialogFilter[]
  /**
   * Web build: also offer uploading from the browser's device. Uploads are
   * staged on the server and removed after a day, so only set this when the
   * caller copies the picked files (imports), never for a location the app
   * keeps using (opening or creating a project). Ignored on desktop.
   */
  allowUpload?: boolean
}

/** Mirrors plugin-dialog: `multiple: true` yields an array. */
export type OpenDialogResult<T extends OpenDialogOptions> = T["multiple"] extends true
  ? string[] | null
  : string | null

export interface SaveDialogOptions {
  title?: string
  defaultPath?: string
  filters?: DialogFilter[]
}

export interface MessageDialogOptions {
  title?: string
  kind?: "info" | "warning" | "error"
}

/** Subset of plugin-store's `Store` the app actually uses. */
export interface KeyValueStore {
  get<T>(key: string): Promise<T | undefined>
  set(key: string, value: unknown): Promise<void>
  delete(key: string): Promise<boolean>
  save(): Promise<void>
}

export interface Platform {
  kind: PlatformKind
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>
  listen<T>(event: string, handler: EventHandler<T>): Promise<UnlistenFn>
  /** URL the webview/browser can load to display a local file. */
  convertFileSrc(path: string): string
  /** fetch() that is not subject to browser CORS for user-configured endpoints. */
  createHttpFetch(): Promise<typeof globalThis.fetch>
  loadStore(name: string): Promise<KeyValueStore>
  openDialog<T extends OpenDialogOptions>(options: T): Promise<OpenDialogResult<T>>
  saveDialog(options: SaveDialogOptions): Promise<string | null>
  messageDialog(message: string, options?: MessageDialogOptions): Promise<void>
  openUrl(url: string): Promise<void>
  openPath(path: string): Promise<void>
  isAutostartEnabled(): Promise<boolean>
  setAutostart(enabled: boolean): Promise<void>
  setWindowTheme(theme: "light" | "dark", background: string): void
}
