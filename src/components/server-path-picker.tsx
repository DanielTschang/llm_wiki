/**
 * File/folder picker for the web build: browses the server's allowed
 * folders, or uploads from this device into the server's staging area.
 * Resolves `requestServerPath` (stores/path-picker-store.ts) with server
 * paths, so call sites written for the desktop dialogs work unchanged.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { ArrowUp, File as FileIcon, Folder, FolderPlus, HardDrive, Upload } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { createDirectory, listDirectory } from "@/commands/fs"
import { invoke } from "@/platform"
import { uploadFiles, type UploadProgress } from "@/lib/server-upload"
import { usePathPickerStore, type PathPickerRequest, type PathPickerResult } from "@/stores/path-picker-store"
import type { FileNode } from "@/types/wiki"
import { cn } from "@/lib/utils"

interface ServerLocations {
  roots: string[]
  uploads: string
}

/** Last folder browsed, so consecutive picks start where the user was. */
let lastDirectory: string | null = null

function joinPath(dir: string, name: string): string {
  return `${dir.replace(/\/+$/, "")}/${name.replace(/^\/+/, "")}`
}

function parentPath(path: string): string {
  const trimmed = path.replace(/\/+$/, "")
  const index = trimmed.lastIndexOf("/")
  return index <= 0 ? "/" : trimmed.slice(0, index)
}

function baseName(path: string | undefined): string {
  return path ? path.replace(/\/+$/, "").split("/").pop() ?? "" : ""
}

export function ServerPathPickerHost() {
  const current = usePathPickerStore((state) => state.current)
  const settle = usePathPickerStore((state) => state.settle)
  if (!current) return null
  return (
    <ServerPathPicker
      key={current.id}
      request={current.request}
      onDone={(result) => settle(current.id, result)}
    />
  )
}

function ServerPathPicker({
  request,
  onDone,
}: {
  request: PathPickerRequest
  onDone: (result: PathPickerResult) => void
}) {
  const { t } = useTranslation()
  const isOpen = request.mode === "open"
  const wantsDirectory = isOpen && !!request.directory
  const multiple = isOpen && !!request.multiple
  const canCreateFolder = !isOpen || wantsDirectory || (isOpen && !!request.createDirectories)
  const extensions = useMemo(
    () => new Set((request.filters ?? []).flatMap((filter) => filter.extensions.map((ext) => ext.toLowerCase()))),
    [request.filters],
  )

  const [tab, setTab] = useState<"server" | "device">("server")
  const [locations, setLocations] = useState<ServerLocations | null>(null)
  const [cwd, setCwd] = useState<string | null>(null)
  const [pathInput, setPathInput] = useState("")
  const [entries, setEntries] = useState<FileNode[]>([])
  const [selected, setSelected] = useState<string[]>([])
  const [fileName, setFileName] = useState(request.mode === "save" ? baseName(request.defaultPath) : "")
  const [newFolderName, setNewFolderName] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [upload, setUpload] = useState<UploadProgress | null>(null)
  const fileInput = useRef<HTMLInputElement>(null)

  const isRoot = cwd !== null && (locations?.roots ?? []).some((root) => root === cwd)

  const navigate = useCallback(async (dir: string | null) => {
    setError(null)
    setSelected([])
    setNewFolderName(null)
    if (dir === null) {
      setCwd(null)
      setPathInput("")
      setEntries([])
      return
    }
    setLoading(true)
    try {
      const nodes = await listDirectory(dir, { maxDepth: 1 })
      setEntries(nodes)
      setCwd(dir)
      setPathInput(dir)
      lastDirectory = dir
    } catch (err) {
      setError(t("serverPicker.loadFailed", { error: String(err) }))
    } finally {
      setLoading(false)
    }
  }, [t])

  useEffect(() => {
    let cancelled = false
    invoke<ServerLocations>("server_locations")
      .then((found) => {
        if (cancelled) return
        setLocations(found)
        const hinted = request.defaultPath
          ? (isOpen && !request.directory ? parentPath(request.defaultPath) : request.defaultPath)
          : null
        const start = [hinted, lastDirectory, found.roots[0]].find(
          (dir) => dir && found.roots.some((root) => dir === root || dir.startsWith(`${root}/`)),
        )
        void navigate(start ?? null)
      })
      .catch((err) => setError(String(err)))
    return () => {
      cancelled = true
    }
  }, [isOpen, navigate, request])

  const visibleEntries = useMemo(() => {
    const matches = (node: FileNode) => {
      if (node.is_dir) return true
      if (wantsDirectory) return false
      if (extensions.size === 0) return true
      const ext = node.name.split(".").pop()?.toLowerCase() ?? ""
      return extensions.has(ext)
    }
    return entries
      .filter(matches)
      .sort((a, b) => Number(b.is_dir) - Number(a.is_dir) || a.name.localeCompare(b.name))
  }, [entries, extensions, wantsDirectory])

  function toggleFile(path: string) {
    setSelected((current) => {
      if (!multiple) return [path]
      return current.includes(path) ? current.filter((p) => p !== path) : [...current, path]
    })
  }

  async function createFolder() {
    if (!cwd || !newFolderName?.trim()) return
    const target = joinPath(cwd, newFolderName.trim())
    try {
      await createDirectory(target)
      await navigate(target)
    } catch (err) {
      setError(String(err))
    }
  }

  async function handleUpload(files: File[]) {
    if (files.length === 0) return
    setError(null)
    try {
      const { batchDir, paths } = await uploadFiles(files, setUpload)
      if (wantsDirectory) {
        const topFolder = files[0].webkitRelativePath.split("/")[0]
        onDone(batchDir && topFolder ? joinPath(batchDir, topFolder) : batchDir)
      } else {
        onDone(multiple ? paths : paths[0] ?? null)
      }
    } catch (err) {
      setUpload(null)
      setError(t("serverPicker.uploadFailed", { error: err instanceof Error ? err.message : String(err) }))
    }
  }

  function confirm() {
    if (request.mode === "save") {
      if (!cwd || !fileName.trim()) return
      let name = fileName.trim()
      const ext = [...extensions][0]
      if (ext && !name.toLowerCase().endsWith(`.${ext}`)) name = `${name}.${ext}`
      onDone(joinPath(cwd, name))
      return
    }
    if (wantsDirectory) {
      onDone(cwd)
      return
    }
    if (selected.length > 0) onDone(multiple ? selected : selected[0])
  }

  const title = request.title
    ?? (request.mode === "save"
      ? t("serverPicker.titleSave")
      : wantsDirectory
        ? t("serverPicker.titleFolder")
        : multiple
          ? t("serverPicker.titleFiles")
          : t("serverPicker.titleFile"))
  const canConfirm = request.mode === "save"
    ? !!cwd && !!fileName.trim()
    : wantsDirectory
      ? !!cwd
      : selected.length > 0
  const confirmLabel = wantsDirectory
    ? t("serverPicker.useFolder")
    : request.mode === "save"
      ? t("serverPicker.save")
      : t("serverPicker.select")

  return (
    <Dialog open onOpenChange={(open) => { if (!open) onDone(null) }}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>

        {isOpen && request.allowUpload && (
          <div className="flex gap-1 rounded-lg bg-muted p-1 text-sm" role="tablist">
            {(["server", "device"] as const).map((value) => (
              <button
                key={value}
                type="button"
                role="tab"
                aria-selected={tab === value}
                onClick={() => setTab(value)}
                className={cn(
                  "flex flex-1 items-center justify-center gap-1.5 rounded-md px-3 py-1.5",
                  tab === value ? "bg-background shadow-sm" : "text-muted-foreground hover:text-foreground",
                )}
              >
                {value === "server" ? <HardDrive className="h-3.5 w-3.5" /> : <Upload className="h-3.5 w-3.5" />}
                {value === "server" ? t("serverPicker.tabServer") : t("serverPicker.tabDevice")}
              </button>
            ))}
          </div>
        )}

        {tab === "server" ? (
          <div className="flex min-w-0 flex-col gap-2">
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="icon-sm"
                disabled={cwd === null}
                onClick={() => void navigate(isRoot ? null : cwd ? parentPath(cwd) : null)}
                aria-label={t("serverPicker.up")}
                title={t("serverPicker.up")}
              >
                <ArrowUp />
              </Button>
              <Input
                value={pathInput}
                placeholder={t("serverPicker.locations")}
                onChange={(event) => setPathInput(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && pathInput.trim()) void navigate(pathInput.trim())
                }}
                aria-label={t("serverPicker.path")}
                className="font-mono text-xs"
              />
              {canCreateFolder && cwd !== null && (
                <Button
                  variant="outline"
                  size="icon-sm"
                  onClick={() => setNewFolderName(newFolderName === null ? "" : null)}
                  aria-label={t("serverPicker.newFolder")}
                  title={t("serverPicker.newFolder")}
                >
                  <FolderPlus />
                </Button>
              )}
            </div>

            {newFolderName !== null && (
              <div className="flex gap-2">
                <Input
                  autoFocus
                  value={newFolderName}
                  placeholder={t("serverPicker.folderName")}
                  onChange={(event) => setNewFolderName(event.target.value)}
                  onKeyDown={(event) => { if (event.key === "Enter") void createFolder() }}
                />
                <Button size="sm" onClick={() => void createFolder()} disabled={!newFolderName.trim()}>
                  {t("serverPicker.create")}
                </Button>
              </div>
            )}

            <div className="h-72 overflow-y-auto rounded-lg border" aria-busy={loading}>
              {cwd === null ? (
                <ul>
                  <li className="px-3 pt-2 pb-1 text-xs font-medium text-muted-foreground">
                    {t("serverPicker.locations")}
                  </li>
                  {(locations?.roots ?? []).map((root) => (
                    <li key={root}>
                      <button
                        type="button"
                        onClick={() => void navigate(root)}
                        className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-muted"
                      >
                        <HardDrive className="h-4 w-4 shrink-0 text-muted-foreground" />
                        <span className="truncate font-mono text-xs">{root}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              ) : visibleEntries.length === 0 && !loading ? (
                <p className="p-4 text-center text-sm text-muted-foreground">{t("serverPicker.empty")}</p>
              ) : (
                <ul>
                  {visibleEntries.map((node) => {
                    const isSelected = selected.includes(node.path)
                    return (
                      <li key={node.path}>
                        <button
                          type="button"
                          onClick={() => (node.is_dir ? void navigate(node.path) : toggleFile(node.path))}
                          onDoubleClick={() => { if (!node.is_dir && !multiple && request.mode === "open") onDone(node.path) }}
                          aria-pressed={node.is_dir ? undefined : isSelected}
                          className={cn(
                            "flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-muted",
                            isSelected && "bg-primary/10 hover:bg-primary/15",
                          )}
                        >
                          {node.is_dir
                            ? <Folder className="h-4 w-4 shrink-0 text-amber-500" />
                            : <FileIcon className="h-4 w-4 shrink-0 text-muted-foreground" />}
                          <span className="truncate">{node.name}</span>
                        </button>
                      </li>
                    )
                  })}
                </ul>
              )}
            </div>

            {request.mode === "save" && (
              <Input
                value={fileName}
                placeholder={t("serverPicker.fileName")}
                onChange={(event) => setFileName(event.target.value)}
                onKeyDown={(event) => { if (event.key === "Enter") confirm() }}
                aria-label={t("serverPicker.fileName")}
              />
            )}
            {multiple && selected.length > 0 && (
              <p className="text-xs text-muted-foreground">{t("serverPicker.selectedCount", { count: selected.length })}</p>
            )}
          </div>
        ) : (
          <div
            className="flex h-72 flex-col items-center justify-center gap-3 rounded-lg border-2 border-dashed p-6 text-center"
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.preventDefault()
              if (!wantsDirectory) void handleUpload(Array.from(event.dataTransfer.files))
            }}
          >
            <Upload className="h-8 w-8 text-muted-foreground" />
            {upload ? (
              <p className="text-sm">{t("serverPicker.uploading", { done: upload.done, total: upload.total })}</p>
            ) : (
              <>
                <p className="text-sm text-muted-foreground">
                  {wantsDirectory ? t("serverPicker.deviceFolderHint") : t("serverPicker.deviceFilesHint")}
                </p>
                <Button variant="outline" size="sm" onClick={() => fileInput.current?.click()}>
                  {wantsDirectory ? t("serverPicker.chooseFolder") : t("serverPicker.chooseFiles")}
                </Button>
              </>
            )}
            <input
              ref={fileInput}
              type="file"
              hidden
              multiple={multiple || wantsDirectory}
              accept={extensions.size > 0 ? [...extensions].map((ext) => `.${ext}`).join(",") : undefined}
              {...(wantsDirectory ? { webkitdirectory: "" } : {})}
              onChange={(event) => void handleUpload(Array.from(event.target.files ?? []))}
            />
          </div>
        )}

        {error && <p className="text-sm text-destructive">{error}</p>}

        <DialogFooter>
          <Button variant="outline" onClick={() => onDone(null)}>{t("common.cancel")}</Button>
          {tab === "server" && (
            <Button onClick={confirm} disabled={!canConfirm}>{confirmLabel}</Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
