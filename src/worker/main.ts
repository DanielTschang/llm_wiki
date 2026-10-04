/**
 * Headless ingest worker for llm-wiki-server (plans/web-server-mode.md,
 * Phase 4). Runs the same ingest queue as the desktop app, in Node, so
 * ingest keeps going with no browser tab open.
 *
 * Built with `npm run build:worker` and started by the server, which
 * passes LLM_WIKI_SERVER_URL and LLM_WIKI_WORKER_TOKEN. Browser tabs never
 * touch the queue directly: they send `ingest_command` RPCs that the server
 * relays here as `ingest-worker://command` events, and they render the
 * `ingest://*` snapshots this worker publishes.
 */
import { invoke, listen } from "@/platform"
import * as queue from "@/lib/ingest-queue"
import { hydrateModelSettings, hydrateProjectModelSettings } from "@/lib/settings-hydration"
import { getLastProject, loadSourceWatchConfig } from "@/lib/project-store"
import { DEFAULT_SOURCE_WATCH_CONFIG } from "@/lib/source-watch-config"
import { useActivityStore } from "@/stores/activity-store"
import { useReviewStore } from "@/stores/review-store"
import { useWikiStore } from "@/stores/wiki-store"
import type { WikiProject } from "@/types/wiki"
import type { IngestQueueSnapshot, IngestWorkerCommand } from "@/lib/ingest-worker-protocol"

const QUEUE_PUBLISH_INTERVAL_MS = 500
const ACTIVITY_PUBLISH_DELAY_MS = 300
const HEARTBEAT_INTERVAL_MS = 10_000
/** Finished activity items kept (and published) once the list grows. */
const MAX_FINISHED_ACTIVITY = 100

/** Queue operations a tab may run here, by name. */
const OPERATIONS: Record<string, (...args: never[]) => unknown> = {
  setIngestWorkerLimit: queue.setIngestWorkerLimit,
  cleanupWrittenFiles: queue.cleanupWrittenFiles,
  enqueueIngest: queue.enqueueIngest,
  enqueueBatch: queue.enqueueBatch,
  enqueueInactiveProjectBatch: queue.enqueueInactiveProjectBatch,
  discardInactiveProjectTasksForSources: queue.discardInactiveProjectTasksForSources,
  retryTask: queue.retryTask,
  retryTasks: queue.retryTasks,
  retryAllFailedTasks: queue.retryAllFailedTasks,
  retryAllStoppedTasks: queue.retryAllStoppedTasks,
  movePendingTask: queue.movePendingTask,
  cancelTask: queue.cancelTask,
  cancelTasks: queue.cancelTasks,
  discardTasksForSources: queue.discardTasksForSources,
  clearCompletedTasks: queue.clearCompletedTasks,
  cancelAllTasks: queue.cancelAllTasks,
  pauseProcessing: queue.pauseProcessing,
  resumeProcessing: queue.resumeProcessing,
  clearQueueState: queue.clearQueueState,
  pauseQueue: queue.pauseQueue,
  // Switching projects also re-applies that project's model settings.
  restoreQueue: (projectId: string, projectPath: string) => activateProject(projectId, projectPath),
  snapshot: () => snapshot(),
}

function snapshot(): IngestQueueSnapshot {
  return {
    projectId: useWikiStore.getState().project?.id ?? "",
    tasks: [...queue.getQueue()],
    summary: queue.getQueueSummary(),
    workerLimit: queue.getIngestWorkerLimit(),
  }
}

async function activateProject(projectId: string, projectPath: string): Promise<void> {
  const name = projectPath.split("/").filter(Boolean).pop() ?? projectPath
  const project: WikiProject = { id: projectId, name, path: projectPath }
  useWikiStore.getState().setProject(project)
  await hydrateProjectModelSettings(projectId)
  try {
    const config = await loadSourceWatchConfig(projectId)
    useWikiStore.getState().setSourceWatchConfig(config)
    queue.setIngestWorkerLimit(config.ingestConcurrency)
  } catch (err) {
    console.error("[worker] failed to load ingest concurrency:", err)
    useWikiStore.getState().setSourceWatchConfig(DEFAULT_SOURCE_WATCH_CONFIG)
    queue.setIngestWorkerLimit(DEFAULT_SOURCE_WATCH_CONFIG.ingestConcurrency)
  }
  await queue.restoreQueue(projectId, projectPath)
  // restoreQueue starts any leftover work right away, so in-flight tasks
  // count as resumed alongside pending ones.
  const { pending, processing } = queue.getQueueSummary()
  const outstanding = pending + processing
  console.log(
    outstanding > 0
      ? `[worker] resuming ${outstanding} interrupted task(s) in ${projectPath}`
      : `[worker] watching ingest queue of ${projectPath} (0 pending)`,
  )
}

function publish(event: string, payload: unknown): void {
  invoke("worker_publish", { event, payload }).catch((err) =>
    console.warn(`[worker] failed to publish ${event}:`, err),
  )
}

async function handleCommand(command: IngestWorkerCommand): Promise<void> {
  const operation = OPERATIONS[command.op]
  try {
    if (!operation) throw new Error(`Unknown ingest operation: ${command.op}`)
    const result = await operation(...(command.args as never[]))
    await invoke("worker_reply", { id: command.id, result: result ?? null })
  } catch (err) {
    await invoke("worker_reply", {
      id: command.id,
      error: err instanceof Error ? err.message : String(err),
    }).catch(() => {})
  }
  publishQueueIfChanged()
}

let lastQueueJson = ""
function publishQueueIfChanged(): void {
  const current = snapshot()
  const json = JSON.stringify(current)
  if (json === lastQueueJson) return
  lastQueueJson = json
  publish("ingest://queue", current)
}

function trimFinishedActivity(): void {
  const { items } = useActivityStore.getState()
  const finished = items.filter((item) => item.status !== "running")
  if (finished.length <= MAX_FINISHED_ACTIVITY) return
  // Newest items come first (addItem prepends).
  const keep = new Set(finished.slice(0, MAX_FINISHED_ACTIVITY))
  useActivityStore.setState({
    items: items.filter((item) => item.status === "running" || keep.has(item)),
  })
}

function forwardActivity(): void {
  let timer: ReturnType<typeof setTimeout> | null = null
  useActivityStore.subscribe(() => {
    if (timer) return
    timer = setTimeout(() => {
      timer = null
      trimFinishedActivity()
      publish("ingest://activity", { items: useActivityStore.getState().items })
    }, ACTIVITY_PUBLISH_DELAY_MS)
  })
}

/**
 * Hands new review items to the server's inbox instead of keeping them:
 * the tab that has the project open adopts them into its own review list,
 * which is the only writer of `review.json`.
 */
function forwardReviewItems(): void {
  useReviewStore.subscribe((state) => {
    if (state.items.length === 0) return
    const items = state.items
    const projectPath = useWikiStore.getState().project?.path
    useReviewStore.setState({ items: [] })
    if (!projectPath) return
    invoke("review_inbox_append", { projectPath, items }).catch((err) =>
      console.error("[worker] failed to hand over review items:", err),
    )
  })
}

let settingsTimer: ReturnType<typeof setTimeout> | null = null
function rehydrateSettingsSoon(): void {
  if (settingsTimer) clearTimeout(settingsTimer)
  settingsTimer = setTimeout(async () => {
    settingsTimer = null
    try {
      await hydrateModelSettings({ persistResolvedPreset: false })
      const project = useWikiStore.getState().project
      if (project) await hydrateProjectModelSettings(project.id)
    } catch (err) {
      console.error("[worker] failed to reload settings:", err)
    }
  }, 500)
}

async function main(): Promise<void> {
  // The server holds our stdin; when it goes away, so do we.
  const proc = (globalThis as {
    process?: { stdin: { on(event: string, listener: () => void): void; resume(): void }; exit(code: number): never }
  }).process
  proc?.stdin.on("end", () => proc.exit(0))
  proc?.stdin.resume()

  await hydrateModelSettings({ persistResolvedPreset: false })
  forwardActivity()
  forwardReviewItems()

  await listen<IngestWorkerCommand>("ingest-worker://command", (event) => {
    void handleCommand(event.payload)
  })
  await listen("app-store://changed", rehydrateSettingsSoon)

  await invoke("worker_heartbeat")
  setInterval(() => {
    invoke("worker_heartbeat").catch((err) => console.warn("[worker] heartbeat failed:", err))
  }, HEARTBEAT_INTERVAL_MS)

  const last = await getLastProject()
  if (last?.id && last.path) {
    try {
      await activateProject(last.id, last.path)
    } catch (err) {
      console.error(`[worker] could not resume ${last.path}:`, err)
    }
  }
  setInterval(publishQueueIfChanged, QUEUE_PUBLISH_INTERVAL_MS)
  console.log("[worker] ready")
}

main().catch((err) => {
  console.error("[worker] fatal:", err)
  ;(globalThis as { process?: { exit(code: number): never } }).process?.exit(1)
})
