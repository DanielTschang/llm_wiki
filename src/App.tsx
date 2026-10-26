import { useState, useEffect } from "react"
import { openDialog } from "@/platform"
import i18n from "@/i18n"
import { useWikiStore } from "@/stores/wiki-store"
import { useReviewStore } from "@/stores/review-store"
import { useLintStore } from "@/stores/lint-store"
import { useChatStore } from "@/stores/chat-store"
import { BASE_FONT_SIZE_PX, useZoomStore } from "@/stores/zoom-store"
import { openProject } from "@/commands/fs"
import { getLastProject, saveLastProject, loadLanguage, loadSourceWatchConfig, loadApiConfig, loadGeneralConfig, loadZoomLevel } from "@/lib/project-store"
import { loadReviewItems, loadLintItems, loadChatHistory, loadChatPreferences } from "@/lib/persist"
import { setupAutoSave } from "@/lib/auto-save"
import { hydrateModelSettings, hydrateProjectModelSettings } from "@/lib/settings-hydration"
import { DEFAULT_SOURCE_WATCH_CONFIG } from "@/lib/source-watch-config"
import { useGlobalShortcut } from "@/hooks/use-global-shortcut"
import { AppLayout } from "@/components/layout/app-layout"
import { WelcomeScreen } from "@/components/project/welcome-screen"
import { CreateProjectDialog } from "@/components/project/create-project-dialog"
import type { WikiProject } from "@/types/wiki"
import { useAppDialog } from "@/stores/app-dialog-store"

function applyDocumentZoom(level: number) {
  document.documentElement.style.fontSize = `${BASE_FONT_SIZE_PX * level}px`
}

function App() {
  const appDialog = useAppDialog()
  const project = useWikiStore((s) => s.project)
  const setProject = useWikiStore((s) => s.setProject)
  const setFileTree = useWikiStore((s) => s.setFileTree)
  const setSelectedFile = useWikiStore((s) => s.setSelectedFile)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const zoomLevel = useZoomStore((s) => s.level)
  const [showCreateDialog, setShowCreateDialog] = useState(false)
  const [loading, setLoading] = useState(true)

  function isCurrentProject(proj: WikiProject): boolean {
    const current = useWikiStore.getState().project
    return current?.id === proj.id && current.path === proj.path
  }

  async function hydrateProjectChatStore(proj: WikiProject): Promise<void> {
    try {
      const activeBeforeLoad = useChatStore.getState().activeConversationId
      const savedChat = await loadChatHistory(proj.path)
      if (!isCurrentProject(proj)) return
      const stateAfterLoad = useChatStore.getState()
      const activeChangedDuringLoad = stateAfterLoad.activeConversationId !== activeBeforeLoad
      if (savedChat.conversations.length > 0) {
        const existingConversations = activeChangedDuringLoad ? stateAfterLoad.conversations : []
        const existingMessages = activeChangedDuringLoad ? stateAfterLoad.messages : []
        const mergedConversations = [
          ...existingConversations,
          ...savedChat.conversations.filter(
            (conversation) => !existingConversations.some((existing) => existing.id === conversation.id)
          ),
        ]
        const mergedMessages = [
          ...existingMessages,
          ...savedChat.messages.filter(
            (message) => !existingMessages.some((existing) => existing.id === message.id)
          ),
        ]
        useChatStore.getState().setConversations(mergedConversations)
        useChatStore.getState().setMessages(mergedMessages)
        const sorted = [...savedChat.conversations].sort((a, b) => b.updatedAt - a.updatedAt)
        if (!activeChangedDuringLoad && sorted[0]) {
          useChatStore.getState().setActiveConversation(sorted[0].id)
        }
      }
    } catch (err) {
      console.warn("[startup] failed to load chat history:", err)
    }
  }

  async function hydrateProjectSideStores(proj: WikiProject): Promise<void> {
    try {
      const savedReview = await loadReviewItems(proj.path)
      if (savedReview.length > 0 && isCurrentProject(proj)) {
        useReviewStore.getState().setItems(savedReview)
      }
    } catch (err) {
      console.warn("[startup] failed to load review items:", err)
    }

    try {
      const savedLint = await loadLintItems(proj.path)
      if (savedLint.length > 0 && isCurrentProject(proj)) {
        useLintStore.getState().setItems(savedLint)
      }
    } catch (err) {
      console.warn("[startup] failed to load lint items:", err)
    }
  }

  // Set up auto-save once on mount
  useEffect(() => {
    setupAutoSave()
  }, [])

  // Register global keyboard shortcuts
  // Cmd+, on macOS or Ctrl+, on Windows/Linux opens settings
  useGlobalShortcut({
    ",": {
      callback: () => setActiveView("settings"),
      allowInTextInput: true,
    },
  })

  useEffect(() => {
    // Apply interface zoom globally, including welcome/settings screens. We
    // scale the rem base instead of using transform: scale() so layout and
    // pointer coordinates remain native; fixed-pixel panels keep their caps.
    applyDocumentZoom(zoomLevel)
  }, [zoomLevel])

  // Dev-only helper for visually testing the update-banner UX.
  // Open dev tools and run:
  //   __llmwiki_testUpdateBanner()
  // to inject a fake "available" result into the update store —
  // banner appears at the top + red dot lights up the gear icon.
  // Run again with arg `false` (or call setDismissed via the store)
  // to clear. Gated on `import.meta.env.DEV` so the helper never
  // ships in production builds.
  useEffect(() => {
    if (!import.meta.env.DEV) return
    ;(async () => {
      const storeMod = await import("@/stores/update-store")
      const { useUpdateStore } = storeMod
      // Expose the live store getter on window so you can inspect
      // state from devtools when debugging banner behavior.
      ;(window as unknown as { __llmwiki_updateStore?: typeof useUpdateStore }).__llmwiki_updateStore = useUpdateStore
      ;(window as unknown as { __llmwiki_testUpdateBanner?: (clear?: boolean) => void }).__llmwiki_testUpdateBanner = (clear = false) => {
        if (clear) {
          useUpdateStore.getState().setResult(
            { kind: "up-to-date", local: __APP_VERSION__, remote: __APP_VERSION__ },
            Date.now(),
          )
          useUpdateStore.getState().setDismissed(null)
          console.log("[test] update banner cleared")
          return
        }
        useUpdateStore.getState().setResult(
          {
            kind: "available",
            local: __APP_VERSION__,
            remote: "v999.0.0",
            release: {
              name: "v999.0.0 (test)",
              tag_name: "v999.0.0",
              body:
                "Test release for banner-UX verification.\n\n" +
                "- Bigger red dot on the Settings icon\n" +
                "- Top banner with one-click dismiss\n" +
                "- Once dismissed, won't reappear for this version",
              html_url: "https://github.com/nashsu/llm_wiki/releases",
              published_at: new Date().toISOString(),
            },
          },
          Date.now(),
        )
        useUpdateStore.getState().setDismissed(null)
        console.log(
          "[test] update banner injected. Run __llmwiki_testUpdateBanner(true) to clear.",
        )
      }
    })()
  }, [])

  // Background update check — hydrate persisted user preferences, then
  // hit GitHub at most once every UPDATE_CHECK_CACHE_MS. Runs 1.5 s
  // after mount so it doesn't contend with the heaviest startup work
  // (project load, file tree, vector store init) but still surfaces
  // a new release in time for the user to notice it during their
  // first interaction. Silent on failure; the UI in Settings → About
  // lets the user retry manually.
  useEffect(() => {
    let cancelled = false
    const timer = setTimeout(async () => {
      if (cancelled) return
      try {
        // A platform server is upgraded by deploying a new version, not from
        // each browser, and must not ask GitHub about a different product.
        const { getWhoAmI } = await import("@/lib/platform-api")
        if (await getWhoAmI()) return
        const { loadUpdateCheckState, saveUpdateCheckState } = await import(
          "@/lib/project-store"
        )
        const { useUpdateStore } = await import("@/stores/update-store")
        const { checkForUpdates, UPDATE_CHECK_CACHE_MS } = await import(
          "@/lib/update-check"
        )

        const persisted = await loadUpdateCheckState()
        if (persisted) useUpdateStore.getState().hydrate(persisted)

        const state = useUpdateStore.getState()
        if (!state.enabled) {
          console.log("[update-check] skipped: user disabled auto-check in settings")
          return
        }

        const now = Date.now()
        // Cache hit requires BOTH the timestamp AND the in-memory
        // result to be present. `lastCheckedAt` is persisted to
        // disk but `lastResult` deliberately is not — keeping the
        // GitHub payload out of the persisted store keeps disk
        // size + privacy footprint small. The downside: a fresh
        // cold start has `lastResult === null` even when
        // `lastCheckedAt` is recent, in which case we MUST refetch
        // — otherwise we'd skip the check AND have no result to
        // display, leaving the banner permanently stuck off.
        // (This was the user-reported bug: "kind=none, no banner".)
        const fresh =
          state.lastCheckedAt !== null &&
          state.lastResult !== null &&
          now - state.lastCheckedAt < UPDATE_CHECK_CACHE_MS
        if (fresh) {
          const ageMin = Math.round((now - (state.lastCheckedAt ?? 0)) / 60_000)
          console.log(
            `[update-check] skipped: cache hit (last check ${ageMin} min ago, ` +
              `cache window ${UPDATE_CHECK_CACHE_MS / 60_000} min). ` +
              `Last result: kind=${state.lastResult?.kind ?? "none"}`,
          )
          return
        }

        useUpdateStore.getState().setChecking(true)
        console.log(
          `[update-check] fetching GitHub releases (local=${__APP_VERSION__})`,
        )
        const result = await checkForUpdates({
          currentVersion: __APP_VERSION__,
          repo: "nashsu/llm_wiki",
        })
        if (cancelled) return
        useUpdateStore.getState().setResult(result, Date.now())
        if (result.kind === "available") {
          console.log(
            `[update-check] update available: local=${result.local} → remote=${result.remote}`,
          )
        } else if (result.kind === "up-to-date") {
          console.log(
            `[update-check] up to date: local=${result.local}, remote latest=${result.remote}`,
          )
        } else {
          console.log(`[update-check] error: ${result.message}`)
        }
        await saveUpdateCheckState({
          enabled: useUpdateStore.getState().enabled,
          lastCheckedAt: Date.now(),
          dismissedVersion: useUpdateStore.getState().dismissedVersion,
        })
      } catch {
        // Silent — Settings → About lets the user retry manually.
      }
    }, 1500)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [])

  // Auto-open last project on startup
  useEffect(() => {
    async function init() {
      try {
        const savedZoom = await loadZoomLevel()
        applyDocumentZoom(savedZoom)
        useZoomStore.getState().setLevel(savedZoom)

        await hydrateModelSettings({ persistResolvedPreset: true })
        // Local HTTP API server config — global (single token + enable
        // flag for the whole install, not per-project). The Rust side
        // reads `apiConfig.{enabled,token,mcpEnabled,allowLanAccess}` from `app-state.json`
        // directly; this only hydrates the Zustand store so the
        // Settings UI reflects the persisted values.
        const savedApi = await loadApiConfig()
        if (savedApi) {
          useWikiStore.getState().setApiConfig({
            enabled: typeof savedApi.enabled === "boolean" ? savedApi.enabled : true,
            allowUnauthenticated:
              typeof savedApi.allowUnauthenticated === "boolean"
                ? savedApi.allowUnauthenticated
                : false,
            allowLanAccess:
              typeof savedApi.allowLanAccess === "boolean" ? savedApi.allowLanAccess : false,
            mcpEnabled:
              typeof savedApi.mcpEnabled === "boolean"
                ? savedApi.mcpEnabled
                : false,
            token: typeof savedApi.token === "string" ? savedApi.token : "",
          })
        }
        const savedGeneral = await loadGeneralConfig()
        useWikiStore.getState().setGeneralConfig(savedGeneral)
        const savedLang = await loadLanguage()
        if (savedLang) {
          await i18n.changeLanguage(savedLang)
        }
        const lastProject = await getLastProject()
        if (lastProject) {
          try {
            const proj = await openProject(lastProject.path)
            await handleProjectOpened(proj)
          } catch {
            // Last project no longer valid
          }
        }
      } catch {
        // ignore init errors
      } finally {
        setLoading(false)
      }
    }
    init()
  }, [])

  async function handleProjectOpened(proj: WikiProject) {
    // Flush the OUTGOING project's review/lint/chat state to disk and suspend
    // auto-save before reset empties the stores — otherwise the debounced
    // writers would persist empty arrays back over the old project's pending
    // review / deep-research items.
    const { runWithSuspendedAutoSave } = await import("@/lib/auto-save")
    await runWithSuspendedAutoSave(async () => {
      // Clear all per-project state BEFORE loading new project data
      // to prevent cross-project contamination. MUST be awaited so the
      // ingest queue / graph cache are actually cleared before the new
      // project's state is populated.
      const { resetProjectState } = await import("@/lib/reset-project-state")
      await resetProjectState()

      setProject(proj)
      await hydrateProjectModelSettings(proj.id)
      setSelectedFile(null)
      setFileTree([])
      setActiveView("wiki")
      // Bump data version so any cached graphs/views invalidate
      useWikiStore.getState().bumpDataVersion()
      await saveLastProject(proj)

      // Apply the project-specific worker limit before restoring its queue so
      // newly enqueued tasks never start with another project's concurrency.
      const { setIngestWorkerLimit } = await import("@/lib/ingest-queue")
      try {
        const config = await loadSourceWatchConfig(proj.id)
        if (!isCurrentProject(proj)) return
        useWikiStore.getState().setSourceWatchConfig(config)
        setIngestWorkerLimit(config.ingestConcurrency)
      } catch (err) {
        console.error("Failed to load ingest concurrency:", err)
        if (!isCurrentProject(proj)) return
        useWikiStore.getState().setSourceWatchConfig(DEFAULT_SOURCE_WATCH_CONFIG)
        setIngestWorkerLimit(DEFAULT_SOURCE_WATCH_CONFIG.ingestConcurrency)
      }

      // Restore ingest queue (resume interrupted tasks). Keyed by the
      // project's stable UUID so the queue still finds the right project
      // even if the filesystem path changed since the task was enqueued.
      try {
        const { restoreQueue } = await import("@/lib/ingest-queue")
        await restoreQueue(proj.id, proj.path)
      } catch (err) {
        console.error("Failed to restore ingest queue:", err)
      }
      // Same handshake for the dedup-merge queue.
      import("@/lib/dedup-queue").then(({ restoreQueue }) => {
        restoreQueue(proj.id, proj.path).catch((err) =>
          console.error("Failed to restore dedup queue:", err)
        )
      })
      // Load lightweight chat preferences before first paint so the chat
      // controls reflect the user's saved tool toggles. The heavier per-
      // conversation history load is deferred below.
      try {
        const savedChatPreferences = await loadChatPreferences(proj.path)
        useChatStore.getState().setUseWebSearch(savedChatPreferences.useWebSearch)
        useChatStore.getState().setUseAnyTxtSearch(savedChatPreferences.useAnyTxtSearch)
        useChatStore.getState().setAgentMode(savedChatPreferences.agentMode)
        useChatStore.getState().setRetrievalMode(savedChatPreferences.retrievalMode)
        useChatStore.getState().setDisabledSkills(savedChatPreferences.disabledSkills)
      } catch {
        // ignore, start fresh
      }
      // Chat history must be hydrated before auto-save resumes. Otherwise the
      // empty store created by resetProjectState() can be persisted over
      // conversations.json before the deferred loader restores old chats.
      await hydrateProjectChatStore(proj)
    }, () => {
      // If project loading fails after resetProjectState() and before persisted
      // review/lint/chat state has been restored, do not leave auto-save armed
      // against a half-loaded project with empty stores.
      setProject(null)
      setFileTree([])
      setSelectedFile(null)
    })
    // Heavy side-store hydration happens after the project shell is allowed
    // to render. Each write has a stale-project guard so a fast project switch
    // cannot apply old review/lint/chat state to the new project.
    void hydrateProjectSideStores(proj)
  }

  async function handleSelectRecent(proj: WikiProject) {
    try {
      const validated = await openProject(proj.path)
      await handleProjectOpened(validated)
    } catch (err) {
      await appDialog.alert({ message: `Failed to open project: ${err}` })
    }
  }

  async function handleOpenProject() {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "Open Wiki Project",
    })
    if (!selected) return
    try {
      const proj = await openProject(selected)
      await handleProjectOpened(proj)
    } catch (err) {
      await appDialog.alert({ message: `Failed to open project: ${err}` })
    }
  }

  async function handleSwitchProject() {
    // Flush outgoing project's review/lint/chat to disk and suspend auto-save
    // before reset empties the stores. resumeAutoSave() runs when the next
    // project opens via handleProjectOpened.
    const { flushAndSuspendAutoSave } = await import("@/lib/auto-save")
    await flushAndSuspendAutoSave()

    // Clear all per-project state BEFORE flipping back to the welcome screen
    // so old data cannot leak in via any async render pass.
    const { resetProjectState } = await import("@/lib/reset-project-state")
    await resetProjectState()
    setProject(null)
    setFileTree([])
    setSelectedFile(null)
  }

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-background text-muted-foreground">
        Loading...
      </div>
    )
  }

  if (!project) {
    return (
      <>
        <WelcomeScreen
          onCreateProject={() => setShowCreateDialog(true)}
          onOpenProject={handleOpenProject}
          onSelectProject={handleSelectRecent}
        />
        <CreateProjectDialog
          open={showCreateDialog}
          onOpenChange={setShowCreateDialog}
          onCreated={handleProjectOpened}
        />
      </>
    )
  }

  return (
    <>
      <AppLayout onSwitchProject={handleSwitchProject} />
      <CreateProjectDialog
        open={showCreateDialog}
        onOpenChange={setShowCreateDialog}
        onCreated={handleProjectOpened}
      />
    </>
  )
}

export default App
