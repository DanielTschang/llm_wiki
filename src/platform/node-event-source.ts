/**
 * Minimal Server-Sent Events client for the ingest worker. Node 22's
 * `EventSource` is experimental and cannot send an Authorization header,
 * so this reads the stream with `fetch`. Implements only what
 * `createWebPlatform` uses: `onmessage` and `close()`, reconnecting after
 * a dropped connection like a browser `EventSource`.
 */

const RECONNECT_DELAY_MS = 2000

export class NodeEventSource {
  onmessage: ((message: { data: string }) => void) | null = null
  private readonly controller = new AbortController()
  private closed = false

  constructor(
    private readonly url: string,
    private readonly headers: Record<string, string>,
    private readonly fetchImpl: typeof globalThis.fetch = globalThis.fetch.bind(globalThis),
  ) {
    void this.run()
  }

  close(): void {
    this.closed = true
    this.controller.abort()
  }

  private async run(): Promise<void> {
    while (!this.closed) {
      try {
        const response = await this.fetchImpl(this.url, {
          headers: { Accept: "text/event-stream", ...this.headers },
          signal: this.controller.signal,
        })
        if (!response.ok || !response.body) {
          throw new Error(`event stream returned HTTP ${response.status}`)
        }
        await this.read(response.body)
      } catch (err) {
        if (this.closed) return
        console.warn("[events] stream interrupted, reconnecting:", err)
      }
      if (!this.closed) await new Promise((resolve) => setTimeout(resolve, RECONNECT_DELAY_MS))
    }
  }

  private async read(body: ReadableStream<Uint8Array>): Promise<void> {
    const reader = body.getReader()
    const decoder = new TextDecoder()
    let buffer = ""
    for (;;) {
      const { done, value } = await reader.read()
      if (done) return
      buffer += decoder.decode(value, { stream: true })
      let boundary: RegExpExecArray | null
      while ((boundary = /\r?\n\r?\n/.exec(buffer))) {
        const block = buffer.slice(0, boundary.index)
        buffer = buffer.slice(boundary.index + boundary[0].length)
        const data = block
          .split(/\r?\n/)
          .filter((line) => line.startsWith("data:"))
          .map((line) => line.slice(5).replace(/^ /, ""))
          .join("\n")
        if (data) this.onmessage?.({ data })
      }
    }
  }
}
