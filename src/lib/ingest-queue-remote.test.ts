import { beforeEach, describe, expect, it, vi } from "vitest"

const invoke = vi.fn()
const listeners = new Map<string, (event: { payload: unknown }) => void>()

vi.mock("@/platform", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
  listen: vi.fn(async (event: string, handler: (event: { payload: unknown }) => void) => {
    listeners.set(event, handler)
    return () => listeners.delete(event)
  }),
}))
vi.mock("@/lib/project-file-tree-refresh", () => ({ refreshProjectFileTree: vi.fn(async () => {}) }))

import * as local from "./ingest-queue"
import * as remote from "./ingest-queue-remote"
import { refreshProjectFileTree } from "@/lib/project-file-tree-refresh"
import { useActivityStore } from "@/stores/activity-store"
import { reviewIdFor, useReviewStore } from "@/stores/review-store"
import { useWikiStore } from "@/stores/wiki-store"

const PROJECT = { id: "p1", name: "wiki", path: "/srv/projects/wiki" }

function task(id: string, status: local.IngestTask["status"]): local.IngestTask {
  return { id, projectId: "p1", sourcePath: `raw/sources/${id}.md`, folderContext: "", status, addedAt: 0, error: null, retryCount: 0 }
}

function snapshotOf(tasks: local.IngestTask[]) {
  return { projectId: "p1", tasks, summary: { ...local.getQueueSummary(), total: tasks.length }, workerLimit: 2 }
}

describe("remote ingest queue", () => {
  beforeEach(() => {
    invoke.mockReset()
    invoke.mockImplementation(async (_cmd: string, args: { op?: string }) =>
      args?.op === "snapshot" ? snapshotOf([]) : null,
    )
    useWikiStore.getState().setProject(PROJECT)
  })

  it("exports the same API as the in-process queue", () => {
    const runtimeExports = (module: object) => Object.keys(module).sort()
    expect(runtimeExports(remote)).toEqual(runtimeExports(local))
  })

  it("forwards mutations to the worker with positional arguments", async () => {
    await remote.enqueueIngest("p1", "raw/sources/a.pdf", "papers")
    expect(invoke).toHaveBeenCalledWith("ingest_command", {
      op: "enqueueIngest",
      args: ["p1", "raw/sources/a.pdf", "papers"],
    })
  })

  it("serves reads from worker snapshots and refreshes the tree when a task finishes", async () => {
    remote.getQueue()
    await vi.waitFor(() => expect(listeners.has("ingest://queue")).toBe(true))
    const publish = listeners.get("ingest://queue")!

    publish({ payload: snapshotOf([task("t1", "processing")]) })
    expect(remote.getQueue().map((t) => t.status)).toEqual(["processing"])
    expect(remote.getIngestWorkerLimit()).toBe(2)
    expect(refreshProjectFileTree).not.toHaveBeenCalled()

    publish({ payload: snapshotOf([task("t1", "done")]) })
    expect(refreshProjectFileTree).toHaveBeenCalledWith(PROJECT.path, { projectId: "p1", bumpDataVersion: true })
  })

  it("mirrors worker activity under a prefix without touching local items", async () => {
    remote.getQueue()
    await vi.waitFor(() => expect(listeners.has("ingest://activity")).toBe(true))
    useActivityStore.setState({ items: [{ id: "activity-1", type: "query", title: "local", status: "running", detail: "", filesWritten: [], createdAt: 1 } as never] })

    listeners.get("ingest://activity")!({ payload: { items: [{ id: "activity-1", title: "ingest a.pdf", status: "running" }] } })

    expect(useActivityStore.getState().items.map((item) => item.id)).toEqual(["worker:activity-1", "activity-1"])
  })

  it("adopts review items for the open project from the server inbox", async () => {
    const item = { id: "worker-side-id", type: "confirm" as const, title: "Check claim", description: "", options: [] }
    invoke.mockImplementation(async (cmd: string, args: { op?: string }) => {
      if (cmd === "review_inbox_take") return [item]
      return args?.op === "snapshot" ? snapshotOf([]) : null
    })
    useReviewStore.setState({ items: [] })

    await remote.restoreQueue("p1", PROJECT.path)

    expect(invoke).toHaveBeenCalledWith("review_inbox_take", { projectPath: PROJECT.path })
    // Re-keyed by content, so a review the owner already resolved stays resolved.
    expect(useReviewStore.getState().items.map((i) => i.id)).toEqual([reviewIdFor(item)])
  })
})
