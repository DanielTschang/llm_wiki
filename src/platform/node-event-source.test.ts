import { describe, expect, it, vi } from "vitest"
import { NodeEventSource } from "./node-event-source"

function streamOf(chunks: string[]): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder()
  return new ReadableStream({
    start(controller) {
      for (const chunk of chunks) controller.enqueue(encoder.encode(chunk))
      controller.close()
    },
  })
}

describe("NodeEventSource", () => {
  it("parses data frames split across chunks and skips keep-alive comments", async () => {
    let calls = 0
    const fetchImpl = vi.fn(async () => {
      calls += 1
      if (calls > 1) return new Promise<Response>(() => {})
      return new Response(streamOf([':\n\ndata: {"event":"a",', '"payload":1}\n\n', "data: second\r\n\r\n"]))
    })
    const messages: string[] = []
    const source = new NodeEventSource("http://server/events", { Authorization: "Bearer t" }, fetchImpl as typeof fetch)
    source.onmessage = (message) => messages.push(message.data)

    await vi.waitFor(() => expect(messages).toHaveLength(2))
    source.close()

    expect(messages).toEqual(['{"event":"a","payload":1}', "second"])
    const [, init] = fetchImpl.mock.calls[0] as unknown as [string, RequestInit]
    expect(init.headers).toMatchObject({ Authorization: "Bearer t", Accept: "text/event-stream" })
  })
})
