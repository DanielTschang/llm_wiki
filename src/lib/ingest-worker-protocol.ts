/**
 * Messages between browser tabs and the server-side ingest worker
 * (src/worker/main.ts) in the web build.
 */
import type { IngestTask, getQueueSummary } from "./ingest-queue"

export type IngestQueueSummary = ReturnType<typeof getQueueSummary>

/** Published by the worker as `ingest://queue` and returned by `snapshot`. */
export interface IngestQueueSnapshot {
  projectId: string
  tasks: IngestTask[]
  summary: IngestQueueSummary
  workerLimit: number
}

/** Relayed by the server as `ingest-worker://command`. */
export interface IngestWorkerCommand {
  id: string
  op: string
  /** Positional arguments of the ingest-queue function named by `op`. */
  args: unknown[]
}
