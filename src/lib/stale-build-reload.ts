/**
 * Web build: when the server has been updated since this tab loaded, the
 * tab's lazily loaded chunks (PDF viewer, Mermaid, …) no longer exist and
 * Vite reports `vite:preloadError`. Reload once to pick up the new build
 * instead of showing "Failed to fetch dynamically imported module".
 *
 * A second failure soon after a reload means the chunk is genuinely broken,
 * so the error is left to the caller rather than reloading in a loop.
 */

const RELOAD_MARKER = "llm-wiki:stale-build-reload"
const LOOP_WINDOW_MS = 30_000

interface ReloadHost {
  addEventListener(type: "vite:preloadError", listener: (event: Event) => void): void
  location: { reload(): void }
  sessionStorage: Pick<Storage, "getItem" | "setItem">
}

export function installStaleBuildReload(host: ReloadHost = window, now: () => number = Date.now): void {
  host.addEventListener("vite:preloadError", (event) => {
    let lastReload = 0
    try {
      lastReload = Number(host.sessionStorage.getItem(RELOAD_MARKER) ?? 0)
    } catch {
      // Storage unavailable (private mode, blocked): still reload once.
    }
    if (now() - lastReload < LOOP_WINDOW_MS) return
    try {
      host.sessionStorage.setItem(RELOAD_MARKER, String(now()))
    } catch {
      // Without storage a loop guard is impossible; reloading once is still
      // the best recovery.
    }
    event.preventDefault()
    host.location.reload()
  })
}
