import { beforeEach, describe, expect, it, vi } from "vitest"

const gatewayFetch = vi.fn<(url: string, init?: RequestInit) => Promise<Response>>()
const createGatewayFetch = vi.fn((_modelId: string, _projectId?: string) => gatewayFetch)
const directFetch = vi.fn<(url: string, init?: RequestInit) => Promise<Response>>()

vi.mock("@/platform", () => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  loadStore: vi.fn(),
  createHttpFetch: vi.fn(),
  createGatewayFetch: (modelId: string, projectId?: string) => createGatewayFetch(modelId, projectId),
}))
vi.mock("./tauri-fetch", async () => {
  const actual = await vi.importActual<typeof import("./tauri-fetch")>("./tauri-fetch")
  return { ...actual, getHttpFetch: () => Promise.resolve(directFetch) }
})

import { streamChat } from "./llm-client"
import { useWikiStore, type LlmConfig } from "@/stores/wiki-store"

const serverModel: LlmConfig = {
  serverModelId: "gw-model",
  provider: "custom",
  apiKey: "",
  model: "m-1",
  ollamaUrl: "",
  customEndpoint: "https://gw.example/v1",
  maxContextSize: 100_000,
  apiMode: "chat_completions",
}

function sse(...tokens: string[]): Response {
  const body = tokens.map((t) => `data: ${JSON.stringify({ choices: [{ delta: { content: t } }] })}\n\n`).join("") + "data: [DONE]\n\n"
  return new Response(body, { status: 200, headers: { "Content-Type": "text/event-stream" } })
}

beforeEach(() => {
  gatewayFetch.mockReset()
  createGatewayFetch.mockClear()
  directFetch.mockReset()
  useWikiStore.setState({ project: { id: "proj-1", name: "P", path: "/projects/proj-1" } })
})

describe("streamChat on a server model", () => {
  it("goes through the gateway, not straight to the provider", async () => {
    gatewayFetch.mockResolvedValue(sse("Hel", "lo"))
    const onToken = vi.fn()
    const onDone = vi.fn()
    await streamChat(serverModel, [{ role: "user", content: "hi" }], { onToken, onDone, onError: vi.fn() })

    expect(directFetch).not.toHaveBeenCalled()
    expect(createGatewayFetch).toHaveBeenCalledWith("gw-model", "proj-1")
    const [target, init] = gatewayFetch.mock.calls[0]
    expect(target).toBe("https://gw.example/v1/chat/completions")
    expect(JSON.parse(String(init?.body))).toMatchObject({ model: "m-1" })
    expect(onToken.mock.calls.map((c) => c[0]).join("")).toBe("Hello")
    expect(onDone).toHaveBeenCalled()
  })

  it("sends no key of its own: the server adds it", async () => {
    gatewayFetch.mockResolvedValue(sse("ok"))
    await streamChat(serverModel, [{ role: "user", content: "hi" }], { onToken: vi.fn(), onDone: vi.fn(), onError: vi.fn() })
    const headers = new Headers(gatewayFetch.mock.calls[0][1]?.headers)
    // Whatever the app puts in the auth header carries no secret.
    expect((headers.get("authorization") ?? "").replace(/^Bearer\s*/i, "")).toBe("")
  })

  it("reports the gateway's refusal like any provider error", async () => {
    gatewayFetch.mockResolvedValue(new Response(JSON.stringify({ error: "the server has no API key for \"gw-model\"" }), { status: 503 }))
    const onError = vi.fn()
    await streamChat(serverModel, [{ role: "user", content: "hi" }], { onToken: vi.fn(), onDone: vi.fn(), onError })
    expect(onError).toHaveBeenCalledTimes(1)
    expect(String(onError.mock.calls[0][0].message)).toContain("503")
    expect(String(onError.mock.calls[0][0].message)).toContain("no API key")
  })

  it("still calls a plain config directly", async () => {
    directFetch.mockResolvedValue(sse("direct"))
    const onToken = vi.fn()
    await streamChat({ ...serverModel, serverModelId: undefined, apiKey: "sk-own" }, [{ role: "user", content: "hi" }], {
      onToken,
      onDone: vi.fn(),
      onError: vi.fn(),
    })
    expect(gatewayFetch).not.toHaveBeenCalled()
    expect(directFetch).toHaveBeenCalledTimes(1)
    expect(onToken).toHaveBeenCalledWith("direct")
  })
})
