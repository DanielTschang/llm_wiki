/**
 * The models a platform server offers.
 *
 * On a platform server the app does not hold API keys. The server keeps a
 * catalogue of models; the app asks it which exist, builds requests as it
 * always has, and sends them through the server's gateway, which adds the key.
 * A server model reaches the rest of the app as an ordinary `LlmConfig` (or
 * `EmbeddingConfig`) with an empty key and `serverModelId` set.
 */
import { invoke } from "@/platform"
import type { EmbeddingConfig, LlmConfig } from "@/stores/wiki-store"

export type ModelPurpose = "chat" | "embedding"

export interface ServerModel {
  id: string
  label: string
  purpose: ModelPurpose
  provider: LlmConfig["provider"]
  model: string
  baseUrl?: string
  apiMode?: LlmConfig["apiMode"]
  maxContextSize?: number
  azureApiVersion?: string
  azureModelFamily?: LlmConfig["azureModelFamily"]
  streaming?: boolean
  vision?: boolean
  customHeaders?: Record<string, string>
  outputDimensionality?: number
  default?: boolean
  /** Only an administrator is sent these two. */
  apiKeyEnv?: string
  keyConfigured?: boolean
}

export function listServerModels(): Promise<ServerModel[]> {
  return invoke<ServerModel[]>("list_llm_models")
}

export function saveServerModels(models: ServerModel[]): Promise<void> {
  // The key status is the server's to work out; it is not part of a model.
  const clean = models.map(({ keyConfigured: _keyConfigured, ...model }) => model)
  return invoke<void>("set_llm_models", { models: clean })
}

const DEFAULT_CONTEXT_CHARS = 204_800

/** The `LlmConfig` that makes the app call `model` through the gateway. */
export function chatConfigFor(model: ServerModel, base?: Partial<LlmConfig>): LlmConfig {
  const baseUrl = model.baseUrl ?? ""
  return {
    ...base,
    serverModelId: model.id,
    provider: model.provider,
    apiKey: "",
    model: model.model,
    ollamaUrl: model.provider === "ollama" ? baseUrl : "",
    customEndpoint: model.provider === "ollama" ? "" : baseUrl,
    azureApiVersion: model.azureApiVersion,
    azureModelFamily: model.azureModelFamily,
    maxContextSize: model.maxContextSize ?? base?.maxContextSize ?? DEFAULT_CONTEXT_CHARS,
    apiMode: model.apiMode,
    streamingEnabled: model.streaming,
    customHeaders: model.customHeaders,
  }
}

/** The `EmbeddingConfig` for a server embedding model. */
export function embeddingConfigFor(model: ServerModel, base?: Partial<EmbeddingConfig>): EmbeddingConfig {
  return {
    ...base,
    serverModelId: model.id,
    enabled: true,
    endpoint: model.baseUrl ?? "",
    apiKey: "",
    model: model.model,
    outputDimensionality: model.outputDimensionality,
  }
}

/** The model to use when the user has not chosen one: the marked default, else the first. */
export function pickModel(
  models: ServerModel[],
  purpose: ModelPurpose,
  preferredId?: string | null,
): ServerModel | undefined {
  const ofPurpose = models.filter((m) => m.purpose === purpose)
  return (
    ofPurpose.find((m) => m.id === preferredId) ??
    ofPurpose.find((m) => m.default) ??
    ofPurpose[0]
  )
}
