import { describe, expect, it, vi } from "vitest"
import { createWebPlatform } from "./web"

class FakeEventSource {
  onmessage: ((message: { data: string }) => void) | null = null
  closed = false
  constructor(readonly url: string) {}
  close() {
    this.closed = true
  }
  emit(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) })
  }
}

function setup(responses: Array<{ status: number; body: string }> = []) {
  const fetch = vi.fn(async () => {
    const next = responses.shift() ?? { status: 200, body: "" }
    return new Response(next.status === 204 ? null : next.body, { status: next.status })
  })
  const sources: FakeEventSource[] = []
  const platform = createWebPlatform({
    baseUrl: "https://wiki.example/",
    fetch: fetch as unknown as typeof globalThis.fetch,
    createEventSource: (url) => {
      const source = new FakeEventSource(url)
      sources.push(source)
      return source as unknown as EventSource
    },
  })
  return { platform, fetch, sources }
}

describe("web platform", () => {
  it("posts invoke args as JSON to /rpc/<command> and returns the JSON result", async () => {
    const { platform, fetch } = setup([{ status: 200, body: JSON.stringify(["a.md"]) }])

    const result = await platform.invoke<string[]>("list_directory", { path: "/p/wiki" })

    expect(result).toEqual(["a.md"])
    expect(fetch).toHaveBeenCalledWith("https://wiki.example/rpc/list_directory", expect.objectContaining({
      method: "POST",
      body: JSON.stringify({ path: "/p/wiki" }),
    }))
  })

  it("marks RPC calls with the client header the server requires", async () => {
    const { platform, fetch } = setup([{ status: 200, body: "null" }])
    await platform.invoke("file_exists", { path: "/p" })
    const [, init] = fetch.mock.calls[0] as unknown as [string, RequestInit]
    expect(init.headers).toMatchObject({ "X-LLM-Wiki-Client": "web" })
  })

  it("reports an expired session before rejecting", async () => {
    const onUnauthorized = vi.fn()
    const fetch = vi.fn(async () => new Response(JSON.stringify({ error: "Not signed in" }), { status: 401 }))
    const platform = createWebPlatform({
      baseUrl: "",
      fetch: fetch as unknown as typeof globalThis.fetch,
      createEventSource: () => ({}) as EventSource,
      onUnauthorized,
    })
    await expect(platform.invoke("file_exists")).rejects.toBe("Not signed in")
    expect(onUnauthorized).toHaveBeenCalledOnce()
  })

  it("resolves undefined for an empty success body", async () => {
    const { platform } = setup([{ status: 204, body: "" }])
    await expect(platform.invoke("write_file", {})).resolves.toBeUndefined()
  })

  it("rejects with the command's error string, like a Tauri command Err", async () => {
    const { platform } = setup([{ status: 400, body: JSON.stringify({ error: "File not found" }) }])
    await expect(platform.invoke("read_file", { path: "/x" })).rejects.toBe("File not found")
  })

  it("keeps a non-JSON error body as the rejection message", async () => {
    const { platform } = setup([{ status: 502, body: "Bad Gateway" }])
    await expect(platform.invoke("read_file")).rejects.toBe("Bad Gateway")
  })

  it("dispatches SSE messages by event name over one shared connection", async () => {
    const { platform, sources } = setup()
    const queue = vi.fn()
    const changed = vi.fn()

    const unlistenQueue = await platform.listen("file-sync://queue-updated", queue)
    const unlistenChanged = await platform.listen("file-sync://changed", changed)
    sources[0].emit({ event: "file-sync://changed", payload: { tasks: [] } })

    expect(sources).toHaveLength(1)
    expect(sources[0].url).toBe("https://wiki.example/events")
    expect(queue).not.toHaveBeenCalled()
    expect(changed).toHaveBeenCalledWith({ event: "file-sync://changed", payload: { tasks: [] } })

    unlistenQueue()
    expect(sources[0].closed).toBe(false)
    unlistenChanged()
    expect(sources[0].closed).toBe(true)
  })

  it("serves files and proxies HTTP through the server", async () => {
    const { platform, fetch } = setup()
    expect(platform.convertFileSrc("/p/raw/a b.png")).toBe(
      "https://wiki.example/files?path=%2Fp%2Fraw%2Fa%20b.png",
    )

    const httpFetch = await platform.createHttpFetch()
    await httpFetch("https://api.example/v1/chat", {
      method: "POST",
      headers: { Authorization: "Bearer k" },
      danger: { acceptInvalidCerts: true },
    } as RequestInit)

    const [url, init] = fetch.mock.calls[0] as unknown as [string, RequestInit]
    expect(url).toBe("https://wiki.example/proxy?url=https%3A%2F%2Fapi.example%2Fv1%2Fchat")
    expect(init).toMatchObject({ method: "POST", credentials: "include" })
    expect(init).not.toHaveProperty("danger")
    const headers = new Headers(init.headers)
    expect(headers.get("Authorization")).toBe("Bearer k")
    expect(headers.get("X-LLM-Wiki-Client")).toBe("web")
  })

  it("keeps app settings on the server via app_store_* commands", async () => {
    const { platform, fetch } = setup([
      { status: 200, body: "null" },
      { status: 200, body: "" },
    ])
    const store = await platform.loadStore("app-state.json")

    await expect(store.get("lastProject")).resolves.toBeUndefined()
    await store.set("lastProject", { path: "/p" })

    expect(fetch).toHaveBeenNthCalledWith(2, "https://wiki.example/rpc/app_store_set", expect.objectContaining({
      body: JSON.stringify({ store: "app-state.json", key: "lastProject", value: { path: "/p" } }),
    }))
  })
})
