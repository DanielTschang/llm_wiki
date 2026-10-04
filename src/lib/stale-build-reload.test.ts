import { describe, expect, it, vi } from "vitest"
import { installStaleBuildReload } from "./stale-build-reload"

function fakeHost() {
  let listener: ((event: Event) => void) | null = null
  const storage = new Map<string, string>()
  const host = {
    addEventListener: (_type: "vite:preloadError", fn: (event: Event) => void) => {
      listener = fn
    },
    location: { reload: vi.fn() },
    sessionStorage: {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => void storage.set(key, value),
    },
  }
  const fire = () => {
    const event = new Event("vite:preloadError", { cancelable: true })
    listener?.(event)
    return event
  }
  return { host, fire }
}

describe("installStaleBuildReload", () => {
  it("reloads once when a lazily loaded chunk is gone", () => {
    const { host, fire } = fakeHost()
    installStaleBuildReload(host, () => 1_000_000)

    const event = fire()

    expect(host.location.reload).toHaveBeenCalledOnce()
    expect(event.defaultPrevented).toBe(true)
  })

  it("does not loop: a failure right after a reload surfaces the error", () => {
    const { host, fire } = fakeHost()
    let clock = 1_000_000
    installStaleBuildReload(host, () => clock)

    fire()
    clock += 5_000
    const second = fire()

    expect(host.location.reload).toHaveBeenCalledOnce()
    expect(second.defaultPrevented).toBe(false)
  })

  it("reloads again for a later deploy", () => {
    const { host, fire } = fakeHost()
    let clock = 1_000_000
    installStaleBuildReload(host, () => clock)

    fire()
    clock += 60 * 60 * 1000
    fire()

    expect(host.location.reload).toHaveBeenCalledTimes(2)
  })
})
