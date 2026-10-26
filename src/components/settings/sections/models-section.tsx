import { useCallback, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { usePlatformInfo } from "@/hooks/use-platform-info"
import { chatConfigFor, listServerModels, saveServerModels, type ServerModel } from "@/lib/llm-gateway"
import { hydratePlatformModels } from "@/lib/settings-hydration"
import { saveActiveServerModelId } from "@/lib/project-store"
import { invoke } from "@/platform"
import { useWikiStore } from "@/stores/wiki-store"

interface UsageRow {
  userId: string | null
  displayName: string | null
  modelId: string
  calls: number
  failed: number
  inputTokens: number
  outputTokens: number
}

const EXAMPLE: ServerModel[] = [
  {
    id: "claude",
    label: "Claude",
    purpose: "chat",
    provider: "anthropic",
    model: "claude-sonnet-5-5",
    apiKeyEnv: "LLM_WIKI_KEY_ANTHROPIC",
    vision: true,
    default: true,
  },
  {
    id: "embeddings",
    label: "Embeddings",
    purpose: "embedding",
    provider: "custom",
    model: "text-embedding-3-small",
    baseUrl: "https://api.openai.com/v1/embeddings",
    apiKeyEnv: "LLM_WIKI_KEY_OPENAI",
    default: true,
  },
]

/** The JSON an administrator edits: the catalogue as stored, without what the
 *  server works out for itself. */
function toEditable(models: ServerModel[]): string {
  return JSON.stringify(
    models.map(({ keyConfigured: _keyConfigured, ...model }) => model),
    null,
    2,
  )
}

export function ModelsSection() {
  const { t } = useTranslation()
  const { isAdmin } = usePlatformInfo()
  const activeId = useWikiStore((s) => s.llmConfig.serverModelId)
  const [models, setModels] = useState<ServerModel[]>([])
  const [loaded, setLoaded] = useState(false)
  const [draft, setDraft] = useState("")
  const [usage, setUsage] = useState<UsageRow[]>([])
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null)

  const reload = useCallback(async () => {
    try {
      const list = await listServerModels()
      setModels(list)
      setDraft(toEditable(list))
      setLoaded(true)
    } catch (error) {
      setMessage({ ok: false, text: String(error) })
    }
  }, [])

  useEffect(() => {
    void reload()
  }, [reload])

  useEffect(() => {
    if (!isAdmin) return
    invoke<UsageRow[]>("llm_usage_summary", { days: 30 })
      .then(setUsage)
      .catch(() => setUsage([]))
  }, [isAdmin])

  async function choose(model: ServerModel) {
    await saveActiveServerModelId(model.id)
    const store = useWikiStore.getState()
    const config = chatConfigFor(model, store.llmConfig)
    store.setLlmConfig(config)
    store.setGlobalLlmConfig(config)
    store.setMultimodalConfig({ ...store.multimodalConfig, enabled: !!model.vision, useMainLlm: true })
  }

  async function save() {
    setMessage(null)
    let parsed: ServerModel[]
    try {
      parsed = JSON.parse(draft) as ServerModel[]
      if (!Array.isArray(parsed)) throw new Error("the list must be a JSON array")
    } catch (error) {
      setMessage({ ok: false, text: String(error) })
      return
    }
    try {
      await saveServerModels(parsed)
      await reload()
      await hydratePlatformModels()
      setMessage({ ok: true, text: t("platform.models.saved") })
    } catch (error) {
      setMessage({ ok: false, text: String(error) })
    }
  }

  const chat = models.filter((m) => m.purpose === "chat")
  const embedding = models.filter((m) => m.purpose === "embedding")

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">{t("platform.models.title")}</h2>
        <p className="mt-1 text-sm text-muted-foreground">{t("platform.models.intro")}</p>
      </div>

      {loaded && models.length === 0 && (
        <p className="rounded-lg border px-4 py-3 text-sm text-muted-foreground">{t("platform.models.noModels")}</p>
      )}

      {chat.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-sm font-medium">{t("platform.models.chat")}</h3>
          <div className="rounded-lg border">
            {chat.map((model) => (
              <div key={model.id} className="flex items-center gap-3 border-b px-4 py-2 last:border-b-0">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium">{model.label}</div>
                  <div className="truncate text-xs text-muted-foreground">
                    {model.provider} · {model.model}
                    {isAdmin && (
                      <span className={model.keyConfigured ? "" : " text-destructive"}>
                        {" · "}
                        {model.keyConfigured ? t("platform.models.keyOk") : t("platform.models.keyMissing")}
                      </span>
                    )}
                  </div>
                </div>
                {activeId === model.id ? (
                  <span className="rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                    {t("platform.models.inUse")}
                  </span>
                ) : (
                  <Button variant="outline" size="sm" onClick={() => void choose(model)}>
                    {t("platform.models.use")}
                  </Button>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {embedding.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-sm font-medium">{t("platform.models.embedding")}</h3>
          <div className="rounded-lg border">
            {embedding.map((model) => (
              <div key={model.id} className="border-b px-4 py-2 last:border-b-0">
                <div className="text-sm font-medium">{model.label}</div>
                <div className="text-xs text-muted-foreground">
                  {model.model} · {t("platform.models.embeddingNote")}
                  {isAdmin && (
                    <span className={model.keyConfigured ? "" : " text-destructive"}>
                      {" · "}
                      {model.keyConfigured ? t("platform.models.keyOk") : t("platform.models.keyMissing")}
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {isAdmin && (
        <div className="space-y-2">
          <h3 className="text-sm font-medium">{t("platform.models.adminTitle")}</h3>
          <p className="text-xs text-muted-foreground">{t("platform.models.adminHelp")}</p>
          <textarea
            aria-label={t("platform.models.adminTitle")}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            spellCheck={false}
            rows={14}
            className="w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs focus:outline-none focus:ring-2 focus:ring-ring"
          />
          <div className="flex gap-2">
            <Button onClick={() => void save()}>{t("platform.models.save")}</Button>
            <Button variant="outline" onClick={() => setDraft(toEditable(EXAMPLE))}>
              {t("platform.models.adminExample")}
            </Button>
            <Button variant="outline" onClick={() => void reload()}>
              {t("platform.models.reload")}
            </Button>
          </div>
          {message && (
            <p className={`text-sm ${message.ok ? "text-muted-foreground" : "text-destructive"}`}>{message.text}</p>
          )}
        </div>
      )}
      {!isAdmin && message && !message.ok && <p className="text-sm text-destructive">{message.text}</p>}

      {isAdmin && (
        <div className="space-y-2">
          <h3 className="text-sm font-medium">{t("platform.models.usageTitle")}</h3>
          {usage.length === 0 ? (
            <p className="text-xs text-muted-foreground">{t("platform.models.usageEmpty")}</p>
          ) : (
            <table className="w-full text-xs">
              <thead className="text-left text-muted-foreground">
                <tr>
                  <th className="py-1 pr-2">{t("platform.models.user")}</th>
                  <th className="py-1 pr-2">{t("platform.models.model")}</th>
                  <th className="py-1 pr-2 text-right">{t("platform.models.calls")}</th>
                  <th className="py-1 pr-2 text-right">{t("platform.models.failed")}</th>
                  <th className="py-1 pr-2 text-right">{t("platform.models.tokensIn")}</th>
                  <th className="py-1 text-right">{t("platform.models.tokensOut")}</th>
                </tr>
              </thead>
              <tbody>
                {usage.map((row) => (
                  <tr key={`${row.userId ?? "-"}:${row.modelId}`} className="border-t">
                    <td className="py-1 pr-2">{row.displayName ?? "—"}</td>
                    <td className="py-1 pr-2">{row.modelId}</td>
                    <td className="py-1 pr-2 text-right">{row.calls}</td>
                    <td className="py-1 pr-2 text-right">{row.failed}</td>
                    <td className="py-1 pr-2 text-right">{row.inputTokens}</td>
                    <td className="py-1 text-right">{row.outputTokens}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
    </div>
  )
}
