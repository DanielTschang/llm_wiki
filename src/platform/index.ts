/**
 * The only module the app should import server capabilities from.
 *
 * The browser build is the default, including under vitest, so tests mock
 * this module. The Node ingest worker build sets VITE_LLM_WIKI_PLATFORM=worker
 * (see `.env.worker`).
 */
import { NodeEventSource } from "./node-event-source"
import { requestServerPath } from "@/stores/path-picker-store"
import { createWebPlatform } from "./web"
import type { Platform } from "./types"

export type {
  DialogFilter,
  KeyValueStore,
  MessageDialogOptions,
  OpenDialogOptions,
  OpenDialogResult,
  PlatformEvent,
  SaveDialogOptions,
  UnlistenFn,
} from "./types"

/** Environment of the Node ingest worker, set by llm-wiki-server. */
function workerEnv(name: string): string {
  const value = (globalThis as { process?: { env: Record<string, string | undefined> } }).process?.env[name]
  if (!value) throw new Error(`${name} is not set; the ingest worker must be started by llm-wiki-server`)
  return value
}

function createWorkerPlatform(): Platform {
  const token = workerEnv("LLM_WIKI_WORKER_TOKEN")
  const fetch = globalThis.fetch.bind(globalThis)
  return createWebPlatform({
    baseUrl: workerEnv("LLM_WIKI_SERVER_URL"),
    fetch,
    authToken: token,
    directFetch: true,
    createEventSource: (url) =>
      new NodeEventSource(url, { Authorization: `Bearer ${token}` }, fetch) as unknown as EventSource,
  })
}

const platform: Platform =
  import.meta.env.VITE_LLM_WIKI_PLATFORM === "worker"
    ? createWorkerPlatform()
    : createWebPlatform({
        baseUrl: import.meta.env.VITE_LLM_WIKI_SERVER_URL ?? "",
        fetch: globalThis.fetch.bind(globalThis),
        createEventSource: (url) => new EventSource(url, { withCredentials: true }),
        onUnauthorized: () => {
          window.location.assign(`${import.meta.env.VITE_LLM_WIKI_SERVER_URL ?? ""}/login`)
        },
        pickPath: requestServerPath,
      })

export const invoke = <T>(command: string, args?: Record<string, unknown>): Promise<T> =>
  platform.invoke<T>(command, args)
export const listen: Platform["listen"] = (event, handler) => platform.listen(event, handler)
export const convertFileSrc = (path: string): string => platform.convertFileSrc(path)
export const createHttpFetch: Platform["createHttpFetch"] = () => platform.createHttpFetch()
export const loadStore: Platform["loadStore"] = (name) => platform.loadStore(name)
export const openDialog: Platform["openDialog"] = (options) => platform.openDialog(options)
export const saveDialog: Platform["saveDialog"] = (options) => platform.saveDialog(options)
export const messageDialog: Platform["messageDialog"] = (text, options) => platform.messageDialog(text, options)
export const openUrl: Platform["openUrl"] = (url) => platform.openUrl(url)
export const openPath: Platform["openPath"] = (path) => platform.openPath(path)
