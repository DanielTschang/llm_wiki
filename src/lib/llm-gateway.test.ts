import { beforeEach, describe, expect, it, vi } from "vitest"

const invoke = vi.fn()
vi.mock("@/platform", () => ({ invoke: (...args: unknown[]) => invoke(...args) }))

import { chatConfigFor, embeddingConfigFor, pickModel, saveServerModels, type ServerModel } from "@/lib/llm-gateway"

const chat = (over: Partial<ServerModel> = {}): ServerModel => ({
  id: "claude",
  label: "Claude",
  purpose: "chat",
  provider: "anthropic",
  model: "claude-x",
  ...over,
})

beforeEach(() => {
  invoke.mockReset()
})

describe("chatConfigFor", () => {
  it("makes an ordinary config with no key and the server's model name", () => {
    const config = chatConfigFor(chat({ apiKeyEnv: "LLM_WIKI_KEY_A" }))
    expect(config).toMatchObject({ serverModelId: "claude", provider: "anthropic", model: "claude-x", apiKey: "" })
    // How the server finds the key is not something the app needs.
    expect(JSON.stringify(config)).not.toContain("LLM_WIKI_KEY")
  })

  it("puts a custom model's address where the app looks for it, and Ollama's elsewhere", () => {
    const custom = chatConfigFor(chat({ provider: "custom", baseUrl: "https://gw.example/v1", apiMode: "anthropic_messages" }))
    expect(custom).toMatchObject({ customEndpoint: "https://gw.example/v1", ollamaUrl: "", apiMode: "anthropic_messages" })
    const ollama = chatConfigFor(chat({ provider: "ollama", baseUrl: "http://localhost:11434" }))
    expect(ollama).toMatchObject({ ollamaUrl: "http://localhost:11434", customEndpoint: "" })
  })

  it("keeps what the user chose that the server does not decide", () => {
    const config = chatConfigFor(chat(), { reasoning: { mode: "high" } as never, maxContextSize: 123 })
    expect(config.reasoning).toEqual({ mode: "high" })
    expect(config.maxContextSize).toBe(123)
    expect(chatConfigFor(chat({ maxContextSize: 5 }), { maxContextSize: 123 }).maxContextSize).toBe(5)
  })

  it("never carries a key over from the config it starts from", () => {
    const config = chatConfigFor(chat(), { apiKey: "sk-old" })
    expect(config.apiKey).toBe("")
  })
})

describe("embeddingConfigFor", () => {
  it("points at the model's address with no key, keeping the chunking choices", () => {
    const model = chat({ purpose: "embedding", provider: "custom", model: "embed-1", baseUrl: "https://gw.example/embeddings", outputDimensionality: 768 })
    const config = embeddingConfigFor(model, { maxChunkChars: 900, apiKey: "sk-old", enabled: false })
    expect(config).toMatchObject({
      serverModelId: "claude",
      enabled: true,
      endpoint: "https://gw.example/embeddings",
      apiKey: "",
      model: "embed-1",
      outputDimensionality: 768,
      maxChunkChars: 900,
    })
  })
})

describe("pickModel", () => {
  const models = [
    chat({ id: "a" }),
    chat({ id: "b", default: true }),
    chat({ id: "e", purpose: "embedding" }),
  ]

  it("prefers the user's choice, then the default, then the first", () => {
    expect(pickModel(models, "chat", "a")?.id).toBe("a")
    expect(pickModel(models, "chat", "gone")?.id).toBe("b")
    expect(pickModel(models, "chat")?.id).toBe("b")
    expect(pickModel([chat({ id: "x" }), chat({ id: "y" })], "chat")?.id).toBe("x")
  })

  it("keeps purposes apart and says so when there is none", () => {
    expect(pickModel(models, "embedding", "a")?.id).toBe("e")
    expect(pickModel([chat()], "embedding")).toBeUndefined()
  })
})

describe("saveServerModels", () => {
  it("sends the list without the key status the server worked out", async () => {
    invoke.mockResolvedValue(undefined)
    await saveServerModels([chat({ keyConfigured: true, apiKeyEnv: "LLM_WIKI_KEY_A" })])
    expect(invoke).toHaveBeenCalledWith("set_llm_models", {
      models: [{ id: "claude", label: "Claude", purpose: "chat", provider: "anthropic", model: "claude-x", apiKeyEnv: "LLM_WIKI_KEY_A" }],
    })
  })
})
