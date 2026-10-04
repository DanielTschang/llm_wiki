/**
 * Browser implementation backed by the self-hosted server. The HTTP
 * contract below is what the Phase 3 `server/` binary must serve:
 *
 *   POST {base}/rpc/{command}   JSON args → JSON result; non-2xx body is
 *                               `{ "error": string }` (mirrors a Tauri
 *                               command's `Err(String)`)
 *   GET  {base}/events          SSE, each message `{ event, payload }`
 *   GET  {base}/files?path=…    raw file inside a registered project
 *   *    {base}/proxy?url=…     forwards method/headers/body to `url`
 *                               and streams the response back
 *
 * App settings (`app-state.json`) live on the server and are reached via
 * the `app_store_get` / `app_store_set` / `app_store_delete` commands.
 *
 * The server authenticates the browser with a session cookie set by its
 * `/login` page. `/rpc` and `/proxy` calls also carry CLIENT_HEADER, which
 * a cross-site page cannot add, so they cannot be forged (CSRF).
 *
 * Native dialogs have no browser equivalent for server-side paths: the app
 * injects `pickPath` (the in-app server file browser,
 * components/server-path-picker.tsx). Without it they fall back to
 * `window.prompt` for a path on the server.
 */
import type {
  EventHandler,
  KeyValueStore,
  OpenDialogOptions,
  OpenDialogResult,
  Platform,
  PlatformEvent,
  SaveDialogOptions,
} from "./types"

export const CLIENT_HEADER = "X-LLM-Wiki-Client"

export interface WebPlatformDeps {
  baseUrl: string
  fetch: typeof globalThis.fetch
  createEventSource: (url: string) => EventSource
  /** Called when the server reports the session is missing or expired. */
  onUnauthorized?: () => void
  /** Bearer token instead of the browser session cookie (ingest worker). */
  authToken?: string
  /**
   * Call user-configured endpoints directly instead of through `/proxy`.
   * Only for Node, where there is no CORS to work around.
   */
  directFetch?: boolean
  /** Picks server paths for open/save dialogs. */
  pickPath?: (
    request: ({ mode: "open" } & OpenDialogOptions) | ({ mode: "save" } & SaveDialogOptions),
  ) => Promise<string | string[] | null>
}

export function createWebPlatform(deps: WebPlatformDeps): Platform {
  const base = deps.baseUrl.replace(/\/+$/, "")
  const handlers = new Map<string, Set<EventHandler<unknown>>>()
  let source: EventSource | null = null

  async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const response = await deps.fetch(`${base}/rpc/${encodeURIComponent(command)}`, {
      method: "POST",
      credentials: "include",
      headers: {
        "Content-Type": "application/json",
        [CLIENT_HEADER]: "web",
        ...(deps.authToken ? { Authorization: `Bearer ${deps.authToken}` } : {}),
      },
      body: JSON.stringify(args ?? {}),
    })
    const text = await response.text()
    if (response.status === 401) deps.onUnauthorized?.()
    if (!response.ok) {
      let error = text || `${command} failed with HTTP ${response.status}`
      try {
        const parsed = JSON.parse(text) as { error?: unknown }
        if (typeof parsed.error === "string") error = parsed.error
      } catch {
        // Non-JSON error body (e.g. a reverse proxy's HTML page): keep raw text.
      }
      // Tauri rejects with the command's error string, and call sites
      // rely on `String(error)` being that message.
      throw error
    }
    return (text ? JSON.parse(text) : undefined) as T
  }

  function ensureEventSource(): void {
    if (source) return
    source = deps.createEventSource(`${base}/events`)
    source.onmessage = (message) => {
      let parsed: PlatformEvent<unknown>
      try {
        parsed = JSON.parse(message.data) as PlatformEvent<unknown>
      } catch {
        console.warn("[platform] dropped malformed server event:", message.data)
        return
      }
      for (const handler of handlers.get(parsed.event) ?? []) handler(parsed)
    }
  }

  function closeEventSourceIfIdle(): void {
    if (handlers.size > 0 || !source) return
    source.close()
    source = null
  }

  async function listen<T>(event: string, handler: EventHandler<T>): Promise<() => void> {
    const set = handlers.get(event) ?? new Set()
    const erased = handler as EventHandler<unknown>
    set.add(erased)
    handlers.set(event, set)
    ensureEventSource()
    return () => {
      set.delete(erased)
      if (set.size === 0) handlers.delete(event)
      closeEventSourceIfIdle()
    }
  }

  const fileSrc = (path: string) => `${base}/files?path=${encodeURIComponent(path)}`

  async function loadStore(name: string): Promise<KeyValueStore> {
    return {
      get: <T>(key: string) => invoke<T | null>("app_store_get", { store: name, key }).then((v) => v ?? undefined),
      set: (key, value) => invoke<void>("app_store_set", { store: name, key, value }),
      delete: (key) => invoke<boolean>("app_store_delete", { store: name, key }),
      // The server persists on every write.
      save: async () => {},
    }
  }

  const proxiedFetch: typeof globalThis.fetch = (input, init) => {
    const target = input instanceof Request ? input.url : String(input)
    // plugin-http's `danger` TLS option is meaningless here; the server
    // applies its own proxy/TLS settings.
    const { danger: _danger, ...rest } = (init ?? {}) as RequestInit & { danger?: unknown }
    const headers = new Headers(rest.headers)
    headers.set(CLIENT_HEADER, "web")
    return deps.fetch(`${base}/proxy?url=${encodeURIComponent(target)}`, {
      ...rest,
      headers,
      credentials: "include",
    })
  }

  return {
    kind: "web",
    invoke,
    listen,
    convertFileSrc: fileSrc,
    createHttpFetch: async () => (deps.directFetch ? deps.fetch : proxiedFetch),
    loadStore,
    openDialog: async <T extends OpenDialogOptions>(options: T) => {
      if (deps.pickPath) return (await deps.pickPath({ mode: "open", ...options })) as OpenDialogResult<T>
      const label = options.title ?? (options.directory ? "Folder path on the server" : "File path on the server")
      const value = window.prompt(label, options.defaultPath ?? "")?.trim()
      if (!value) return null as OpenDialogResult<T>
      return (options.multiple ? [value] : value) as OpenDialogResult<T>
    },
    saveDialog: async (options) => {
      if (deps.pickPath) return (await deps.pickPath({ mode: "save", ...options })) as string | null
      const value = window.prompt(options.title ?? "Save to path on the server", options.defaultPath ?? "")?.trim()
      return value || null
    },
    messageDialog: async (text, options) => {
      window.alert(options?.title ? `${options.title}\n\n${text}` : text)
    },
    openUrl: async (url) => {
      window.open(url, "_blank", "noopener,noreferrer")
    },
    // The server's OS file manager is not reachable from the browser;
    // show the file itself instead.
    openPath: async (path) => {
      window.open(fileSrc(path), "_blank", "noopener,noreferrer")
    },
    isAutostartEnabled: async () => false,
    setAutostart: async () => {},
    setWindowTheme: () => {},
  }
}
