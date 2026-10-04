/**
 * Web-build stand-in for `@/lib/ingest-queue` (swapped in by the Vite
 * alias in `--mode web`). Same API, but the queue runs in the server's
 * ingest worker (src/worker/main.ts): reads come from the snapshots it
 * publishes, mutations are forwarded as `ingest_command` RPCs.
 *
 * Keep the exports identical to ingest-queue.ts —
 * ingest-queue-remote.test.ts fails when they drift.
 */
import { invoke, listen } from "@/platform"
import { useActivityStore, type ActivityItem } from "@/stores/activity-store"
import { useReviewStore, type ReviewItem } from "@/stores/review-store"
import { useWikiStore } from "@/stores/wiki-store"
import { refreshProjectFileTree } from "@/lib/project-file-tree-refresh"
import { normalizePath } from "@/lib/path-utils"
import type { IngestTask } from "./ingest-queue"
import type { IngestQueueSnapshot, IngestQueueSummary } from "./ingest-worker-protocol"

export type { IngestTask }

/** Worker activity ids are prefixed so they cannot collide with this tab's. */
const WORKER_ACTIVITY_PREFIX = "worker:"

const EMPTY_SUMMARY: IngestQueueSummary = {
  pending: 0,
  processing: 0,
  failed: 0,
  cancelled: 0,
  completed: 0,
  total: 0,
  paused: false,
  userPaused: false,
  blockedOnLlmConfig: false,
}

let snapshot: IngestQueueSnapshot = { projectId: "", tasks: [], summary: EMPTY_SUMMARY, workerLimit: 1 }
let subscription: Promise<void> | null = null

function command<T>(op: string, ...args: unknown[]): Promise<T> {
  ensureSubscribed()
  return invoke<T>("ingest_command", { op, args })
}

function ensureSubscribed(): void {
  if (subscription) return
  subscription = (async () => {
    await listen<IngestQueueSnapshot>("ingest://queue", (event) => applySnapshot(event.payload))
    await listen<{ items: ActivityItem[] }>("ingest://activity", (event) => applyWorkerActivity(event.payload.items))
    await listen<{ projectPath: string }>("ingest://review-inbox", (event) => {
      void adoptReviewInbox(event.payload.projectPath)
    })
    applySnapshot(await invoke<IngestQueueSnapshot>("ingest_command", { op: "snapshot", args: [] }))
  })().catch((err) => {
    subscription = null
    console.warn("[ingest] cannot reach the server's ingest worker:", err)
  })
}

function applySnapshot(next: IngestQueueSnapshot): void {
  const finished = next.tasks.some((task) => {
    const before = snapshot.tasks.find((old) => old.id === task.id)
    return task.status === "done" && before?.status !== "done"
  })
  snapshot = next
  const project = useWikiStore.getState().project
  // Ingest wrote wiki pages from another process: refresh this tab's view.
  if (finished && project && project.id === next.projectId) {
    void refreshProjectFileTree(project.path, { projectId: project.id, bumpDataVersion: true })
  }
}

function applyWorkerActivity(items: ActivityItem[]): void {
  const mirrored = items.map((item) => ({ ...item, id: `${WORKER_ACTIVITY_PREFIX}${item.id}` }))
  useActivityStore.setState((state) => ({
    items: [
      ...mirrored,
      ...state.items.filter((item) => !item.id.startsWith(WORKER_ACTIVITY_PREFIX)),
    ],
  }))
}

/** Moves review items the worker produced into this tab's review list. */
async function adoptReviewInbox(projectPath: string): Promise<void> {
  const project = useWikiStore.getState().project
  if (!project || normalizePath(project.path) !== normalizePath(projectPath)) return
  try {
    const items = await invoke<ReviewItem[]>("review_inbox_take", { projectPath: project.path })
    if (items.length > 0) useReviewStore.getState().addItems(items)
  } catch (err) {
    console.warn("[ingest] failed to collect review items from the worker:", err)
  }
}

export function setIngestWorkerLimit(limit: number): void {
  snapshot = { ...snapshot, workerLimit: limit }
  command("setIngestWorkerLimit", limit).catch((err) => console.warn("[ingest] setIngestWorkerLimit:", err))
}

export function getIngestWorkerLimit(): number {
  return snapshot.workerLimit
}

export function cleanupWrittenFiles(projectPath: string, filePaths: string[]): Promise<void> {
  return command("cleanupWrittenFiles", projectPath, filePaths)
}

export function enqueueIngest(projectId: string, sourcePath: string, folderContext = ""): Promise<string> {
  return command("enqueueIngest", projectId, sourcePath, folderContext)
}

export function enqueueBatch(
  projectId: string,
  files: Array<{ sourcePath: string; folderContext: string }>,
): Promise<string[]> {
  return command("enqueueBatch", projectId, files)
}

export function enqueueInactiveProjectBatch(
  projectId: string,
  projectPath: string,
  files: Array<{ sourcePath: string; folderContext: string }>,
): Promise<string[]> {
  return command("enqueueInactiveProjectBatch", projectId, projectPath, files)
}

export function discardInactiveProjectTasksForSources(
  projectId: string,
  projectPath: string,
  sourcePaths: readonly string[],
): Promise<number> {
  return command("discardInactiveProjectTasksForSources", projectId, projectPath, sourcePaths)
}

export function retryTask(taskId: string): Promise<void> {
  return command("retryTask", taskId)
}

export function retryTasks(taskIds: readonly string[]): Promise<number> {
  return command("retryTasks", taskIds)
}

export function retryAllFailedTasks(): Promise<number> {
  return command("retryAllFailedTasks")
}

export function retryAllStoppedTasks(): Promise<number> {
  return command("retryAllStoppedTasks")
}

export function movePendingTask(taskId: string, direction: "up" | "down"): Promise<boolean> {
  return command("movePendingTask", taskId, direction)
}

export function cancelTask(taskId: string): Promise<void> {
  return command("cancelTask", taskId)
}

export function cancelTasks(taskIds: readonly string[]): Promise<number> {
  return command("cancelTasks", taskIds)
}

export function discardTasksForSources(sourcePaths: readonly string[]): Promise<number> {
  return command("discardTasksForSources", sourcePaths)
}

export function clearCompletedTasks(): Promise<void> {
  return command("clearCompletedTasks")
}

export function cancelAllTasks(): Promise<number> {
  return command("cancelAllTasks")
}

export function pauseProcessing(): void {
  snapshot = { ...snapshot, summary: { ...snapshot.summary, paused: true, userPaused: true } }
  command("pauseProcessing").catch((err) => console.warn("[ingest] pauseProcessing:", err))
}

export function resumeProcessing(): void {
  snapshot = { ...snapshot, summary: { ...snapshot.summary, userPaused: false } }
  command("resumeProcessing").catch((err) => console.warn("[ingest] resumeProcessing:", err))
}

export function isQueuePaused(): boolean {
  return snapshot.summary.userPaused
}

export function getQueue(): readonly IngestTask[] {
  ensureSubscribed()
  return snapshot.tasks
}

export function getQueueSummary(): IngestQueueSummary {
  ensureSubscribed()
  return snapshot.summary
}

export function clearQueueState(): void {
  snapshot = { ...snapshot, tasks: [], summary: EMPTY_SUMMARY }
  command("clearQueueState").catch((err) => console.warn("[ingest] clearQueueState:", err))
}

export function pauseQueue(): Promise<void> {
  return command("pauseQueue")
}

/** Also called when this tab opens a project: the worker follows it. */
export async function restoreQueue(projectId: string, projectPath: string): Promise<void> {
  await command("restoreQueue", projectId, projectPath)
  applySnapshot(await command<IngestQueueSnapshot>("snapshot"))
  await adoptReviewInbox(projectPath)
}
