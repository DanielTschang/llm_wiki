/**
 * Loads persisted model/ingest settings into the wiki store. Shared by the
 * app shell (App.tsx) and the headless ingest worker (src/worker), so both
 * run ingest with the same configuration.
 */
import { useWikiStore } from "@/stores/wiki-store"
import {
  loadActivePresetId,
  loadCustomLlmPresets,
  loadEmbeddingConfig,
  loadLlmConfig,
  loadMineruConfig,
  loadMultimodalConfig,
  loadOutputLanguage,
  loadProjectLlmOverride,
  loadProviderConfigs,
  loadProxyConfig,
  loadSearchApiConfig,
  loadTaskModelRouting,
  saveLlmConfig,
} from "@/lib/project-store"
import { resolveProjectLlmConfig } from "@/lib/llm-task-routing"
import { getWhoAmI } from "@/lib/platform-api"
import {
  chatConfigFor,
  embeddingConfigFor,
  listServerModels,
  pickModel,
  type ServerModel,
} from "@/lib/llm-gateway"
import { loadActiveServerModelId } from "@/lib/project-store"

export interface HydrateModelSettingsOptions {
  /**
   * Write the re-resolved active preset back to `llmConfig`. Only the app
   * shell does this; the worker must not write settings.
   */
  persistResolvedPreset: boolean
}

/** Points the app at the models a platform server offers: the chosen (or
 *  default) chat model, the embedding model, captions on the chat model if it
 *  can see. Returns false on a server that has no accounts. */
export async function hydratePlatformModels(models?: ServerModel[]): Promise<boolean> {
  if (!(await getWhoAmI())) return false
  const store = useWikiStore.getState()
  const offered = models ?? (await listServerModels().catch(() => []))
  const chat = pickModel(offered, "chat", await loadActiveServerModelId().catch(() => null))
  if (chat) {
    const config = chatConfigFor(chat, store.llmConfig)
    store.setLlmConfig(config)
    store.setGlobalLlmConfig(config)
  }
  const embedding = pickModel(offered, "embedding")
  store.setEmbeddingConfig(
    embedding
      ? embeddingConfigFor(embedding, store.embeddingConfig)
      : { ...store.embeddingConfig, enabled: false, serverModelId: undefined },
  )
  store.setMultimodalConfig({ ...store.multimodalConfig, enabled: !!chat?.vision, useMainLlm: true })
  return true
}

export async function hydrateModelSettings(options: HydrateModelSettingsOptions): Promise<void> {
  const store = () => useWikiStore.getState()

  // On a platform server the models, and the keys behind them, are the
  // server's; nothing about them is read from the settings store.
  if (await hydratePlatformModels()) {
    const savedTaskModelRouting = await loadTaskModelRouting()
    if (savedTaskModelRouting) store().setTaskModelRouting(savedTaskModelRouting)
    return
  }

  const savedConfig = await loadLlmConfig()
  if (savedConfig) {
    store().setLlmConfig(savedConfig)
    store().setGlobalLlmConfig(savedConfig)
  }
  const savedProviderConfigs = await loadProviderConfigs()
  if (savedProviderConfigs) {
    store().setProviderConfigs(savedProviderConfigs)
  }
  const savedCustomLlmPresets = await loadCustomLlmPresets()
  store().setCustomLlmPresets(savedCustomLlmPresets)
  const savedActivePreset = await loadActivePresetId()
  if (savedActivePreset) {
    store().setActivePresetId(savedActivePreset)
    // Re-resolve the active preset's LlmConfig from (preset defaults
    // + saved overrides). Without this, preset default updates
    // (e.g. a corrected Anthropic model ID shipped in a release)
    // never reach users who are relying on defaults — their stored
    // `llmConfig` snapshot from a previous launch would keep the
    // old value. Overrides still win, so an explicit user choice
    // is preserved.
    const { findLlmPreset } = await import("@/components/settings/llm-presets")
    const { resolveConfig } = await import("@/components/settings/preset-resolver")
    const preset = findLlmPreset(savedActivePreset, savedCustomLlmPresets)
    if (preset) {
      const currentFallback = store().llmConfig
      const override = (savedProviderConfigs ?? {})[savedActivePreset]
      const resolved = resolveConfig(preset, override, currentFallback)
      store().setLlmConfig(resolved)
      store().setGlobalLlmConfig(resolved)
      if (options.persistResolvedPreset) await saveLlmConfig(resolved)
    }
  }
  const savedTaskModelRouting = await loadTaskModelRouting()
  if (savedTaskModelRouting) {
    store().setTaskModelRouting(savedTaskModelRouting)
  }
  const savedSearchConfig = await loadSearchApiConfig()
  if (savedSearchConfig) {
    store().setSearchApiConfig(savedSearchConfig)
  }
  const savedEmbeddingConfig = await loadEmbeddingConfig()
  if (savedEmbeddingConfig) {
    store().setEmbeddingConfig(savedEmbeddingConfig)
  }
  const savedMultimodalConfig = await loadMultimodalConfig()
  if (savedMultimodalConfig) {
    store().setMultimodalConfig(savedMultimodalConfig)
  }
  const savedMineruConfig = await loadMineruConfig()
  if (savedMineruConfig) {
    store().setMineruConfig(savedMineruConfig)
  }
  const savedProxy = await loadProxyConfig()
  if (savedProxy) {
    store().setProxyConfig(savedProxy)
  }
}

/**
 * Applies a project's model override and output language on top of the
 * global settings loaded by `hydrateModelSettings`.
 */
export async function hydrateProjectModelSettings(projectId: string): Promise<void> {
  const projectLlmOverride = await loadProjectLlmOverride(projectId)
  const llmState = useWikiStore.getState()
  llmState.setProjectLlmOverride(projectLlmOverride)
  llmState.setLlmConfig(resolveProjectLlmConfig(
    llmState.globalLlmConfig,
    llmState.providerConfigs,
    projectLlmOverride,
    llmState.customLlmPresets,
  ))
  const projectOutputLang = await loadOutputLanguage(projectId)
  useWikiStore.getState().setOutputLanguage(projectOutputLang ?? "auto")
}
