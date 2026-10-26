import { useEffect, useState } from "react"
import { FolderOpen, Plus, Clock, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { getRecentProjects, removeFromRecentProjects } from "@/lib/project-store"
import { listMyProjects, type ServerProject } from "@/lib/platform-api"
import { usePlatformInfo } from "@/hooks/use-platform-info"
import type { WikiProject } from "@/types/wiki"
import { useTranslation } from "react-i18next"

interface WelcomeScreenProps {
  onCreateProject: () => void
  onOpenProject: () => void
  onSelectProject: (project: WikiProject) => void
}

export function WelcomeScreen({
  onCreateProject,
  onOpenProject,
  onSelectProject,
}: WelcomeScreenProps) {
  const { t } = useTranslation()
  const [recentProjects, setRecentProjects] = useState<WikiProject[]>([])
  const platform = usePlatformInfo()
  // On a platform server the projects are the ones the user belongs to.
  const [serverProjects, setServerProjects] = useState<ServerProject[] | null>(null)

  useEffect(() => {
    if (platform.loading) return
    if (platform.isPlatform) {
      listMyProjects().then(setServerProjects).catch(() => setServerProjects([]))
    } else {
      getRecentProjects().then(setRecentProjects).catch(() => {})
    }
  }, [platform.loading, platform.isPlatform])

  async function handleRemoveRecent(e: React.MouseEvent, path: string) {
    e.stopPropagation()
    await removeFromRecentProjects(path)
    const updated = await getRecentProjects()
    setRecentProjects(updated)
  }

  return (
    <div className="flex h-full items-center justify-center bg-background">
      <div className="flex flex-col items-center gap-8 px-4">
        <div className="text-center">
          <h1 className="text-3xl font-bold">{t("app.title")}</h1>
          <p className="mt-2 text-muted-foreground">
            {t("app.subtitle")}
          </p>
        </div>

        <div className="flex gap-3">
          {(!platform.isPlatform || platform.isAdmin) && (
            <Button onClick={onCreateProject}>
              <Plus className="mr-2 h-4 w-4" />
              {t("welcome.newProject")}
            </Button>
          )}
          {!platform.isPlatform && (
            <Button variant="outline" onClick={onOpenProject}>
              <FolderOpen className="mr-2 h-4 w-4" />
              {t("welcome.openProject")}
            </Button>
          )}
        </div>

        {platform.isPlatform && serverProjects && (
          <div className="w-full max-w-md">
            {serverProjects.length === 0 ? (
              <div className="rounded-lg border px-4 py-6 text-center text-sm text-muted-foreground">
                <p>
                  {platform.isAdmin
                    ? t("platform.welcome.adminNoProjects")
                    : t("platform.welcome.noProjects")}
                </p>
                {!platform.isAdmin && <p className="mt-1">{t("platform.welcome.askAdmin")}</p>}
              </div>
            ) : (
              <>
                <div className="mb-2 flex items-center gap-2 text-sm text-muted-foreground">
                  <Clock className="h-3.5 w-3.5" />
                  {t("platform.welcome.projects")}
                </div>
                <div className="rounded-lg border">
                  {serverProjects.map((proj) => (
                    <button
                      key={proj.id}
                      onClick={() => onSelectProject({ id: proj.id, name: proj.name, path: proj.path })}
                      className="flex w-full items-center justify-between border-b px-4 py-3 text-left transition-colors last:border-b-0 hover:bg-accent"
                    >
                      <div className="min-w-0 flex-1 truncate text-sm font-medium">{proj.name}</div>
                      {proj.role && (
                        <span className="ml-2 shrink-0 rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                          {t(`platform.roles.${proj.role}`)}
                        </span>
                      )}
                    </button>
                  ))}
                </div>
              </>
            )}
          </div>
        )}

        {!platform.isPlatform && recentProjects.length > 0 && (
          <div className="w-full max-w-md">
            <div className="mb-2 flex items-center gap-2 text-sm text-muted-foreground">
              <Clock className="h-3.5 w-3.5" />
              {t("welcome.recentProjects")}
            </div>
            <div className="rounded-lg border">
              {recentProjects.map((proj) => (
                <button
                  key={proj.path}
                  onClick={() => onSelectProject(proj)}
                  className="group flex w-full items-center justify-between border-b px-4 py-3 text-left transition-colors last:border-b-0 hover:bg-accent"
                >
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">{proj.name}</div>
                    <div className="truncate text-xs text-muted-foreground">
                      {proj.path}
                    </div>
                  </div>
                  <div
                    role="button"
                    tabIndex={0}
                    onClick={(e) => handleRemoveRecent(e, proj.path)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleRemoveRecent(e as unknown as React.MouseEvent, proj.path)
                    }}
                    className="ml-2 shrink-0 rounded p-1 opacity-0 transition-opacity hover:bg-destructive/10 group-hover:opacity-100"
                  >
                    <X className="h-3.5 w-3.5 text-muted-foreground" />
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
