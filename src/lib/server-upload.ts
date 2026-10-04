/**
 * Uploads files from the browser's device to llm-wiki-server's staging
 * area (`POST /upload`, see crates/server/src/upload.rs) and returns their
 * server paths. Web build only.
 */
import { CLIENT_HEADER } from "@/platform/web"

export interface UploadProgress {
  done: number
  total: number
}

function serverUrl(path: string): string {
  return `${import.meta.env.VITE_LLM_WIKI_SERVER_URL ?? ""}${path}`
}

/** Relative name to store a file under: keeps a picked folder's layout. */
function uploadName(file: File): string {
  return file.webkitRelativePath || file.name
}

/**
 * Uploads `files` into one new batch folder. Returns each file's server
 * path, in order. A folder pick keeps its sub-folders, so the folder itself
 * is `<batch>/<top-level folder>`.
 */
export async function uploadFiles(
  files: readonly File[],
  onProgress?: (progress: UploadProgress) => void,
): Promise<{ batchDir: string | null; paths: string[] }> {
  const batch = crypto.randomUUID()
  const paths: string[] = []
  onProgress?.({ done: 0, total: files.length })
  for (const file of files) {
    const query = new URLSearchParams({ batch, name: uploadName(file) })
    const response = await fetch(serverUrl(`/upload?${query}`), {
      method: "POST",
      credentials: "include",
      headers: { [CLIENT_HEADER]: "web", "Content-Type": "application/octet-stream" },
      body: file,
    })
    const body = (await response.json().catch(() => ({}))) as { path?: string; error?: string }
    if (!response.ok || !body.path) {
      throw new Error(body.error ?? `Upload of ${file.name} failed with HTTP ${response.status}`)
    }
    paths.push(body.path)
    onProgress?.({ done: paths.length, total: files.length })
  }
  const first = paths[0]
  const batchDir = first ? first.slice(0, first.indexOf(`/${batch}/`) + batch.length + 1) : null
  return { batchDir, paths }
}

/** Server URL that makes the browser download a server file. */
export function downloadUrl(path: string): string {
  return serverUrl(`/files?${new URLSearchParams({ path, download: "true" })}`)
}
