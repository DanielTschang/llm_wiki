import { useState } from "react"
import { useTranslation } from "react-i18next"
import { AlertTriangle, PanelLeftClose } from "lucide-react"
import { KnowledgeTree } from "./knowledge-tree"
import { FileTree } from "./file-tree"
import { useWikiStore } from "@/stores/wiki-store"
import { refreshProjectFileTree } from "@/lib/project-file-tree-refresh"

interface SidebarPanelProps {
  onCollapse?: () => void
}

export function SidebarPanel({ onCollapse }: SidebarPanelProps) {
  const { t } = useTranslation()
  const [mode, setMode] = useState<"knowledge" | "files">("knowledge")
  const project = useWikiStore((s) => s.project)
  const fileTreeError = useWikiStore((s) => s.fileTreeError)

  return (
    <div className="flex h-full flex-col">
      <div className="flex shrink-0 border-b">
        <button
          onClick={() => setMode("knowledge")}
          className={`flex-1 px-3 py-1.5 text-xs font-medium transition-colors ${
            mode === "knowledge"
              ? "border-b-2 border-primary text-foreground"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          {t("sidebar.knowledge")}
        </button>
        <button
          onClick={() => setMode("files")}
          className={`flex-1 px-3 py-1.5 text-xs font-medium transition-colors ${
            mode === "files"
              ? "border-b-2 border-primary text-foreground"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          {t("sidebar.files")}
        </button>
        {onCollapse && (
          <button
            type="button"
            onClick={onCollapse}
            className="flex w-9 shrink-0 items-center justify-center border-l text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            title={t("layout.hideSidebar", "Hide sidebar")}
            aria-label={t("layout.hideSidebar", "Hide sidebar")}
          >
            <PanelLeftClose className="h-4 w-4" />
          </button>
        )}
      </div>
      {fileTreeError && project && (
        <div role="alert" className="flex shrink-0 items-start gap-2 border-b bg-amber-500/10 px-3 py-2 text-xs">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" />
          <div className="min-w-0 flex-1 space-y-1">
            <p className="break-words">{t("fileTree.loadFailed", { error: fileTreeError })}</p>
            <button
              type="button"
              onClick={() => void refreshProjectFileTree(project.path, { projectId: project.id })}
              className="font-medium text-primary hover:underline"
            >
              {t("common.retry")}
            </button>
          </div>
        </div>
      )}
      <div className="flex-1 overflow-hidden">
        {mode === "knowledge" ? <KnowledgeTree /> : <FileTree />}
      </div>
    </div>
  )
}
