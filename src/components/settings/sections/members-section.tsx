import { useCallback, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { usePlatformInfo } from "@/hooks/use-platform-info"
import {
  addProjectMember,
  listProjectMembers,
  removeProjectMember,
  setProjectMemberRole,
  type ProjectMember,
  type ProjectRole,
} from "@/lib/platform-api"
import { useWikiStore } from "@/stores/wiki-store"

const ROLES: ProjectRole[] = ["owner", "editor", "viewer"]

export function MembersSection() {
  const { t } = useTranslation()
  const project = useWikiStore((s) => s.project)
  const { user, isAdmin } = usePlatformInfo()
  const [members, setMembers] = useState<ProjectMember[]>([])
  const [username, setUsername] = useState("")
  const [role, setRole] = useState<ProjectRole>("viewer")
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null)

  const projectId = project?.id
  const reload = useCallback(async () => {
    if (!projectId) return
    try {
      setMembers(await listProjectMembers(projectId))
    } catch (error) {
      setMessage({ ok: false, text: String(error) })
    }
  }, [projectId])

  useEffect(() => {
    void reload()
  }, [reload])

  if (!project) {
    return (
      <div className="space-y-2">
        <h2 className="text-xl font-semibold">{t("platform.members.title")}</h2>
        <p className="text-sm text-muted-foreground">{t("platform.members.none")}</p>
      </div>
    )
  }

  const mine = members.find((m) => m.userId === user?.id)
  const canManage = isAdmin || mine?.role === "owner"

  async function run(action: () => Promise<unknown>, success?: string) {
    setMessage(null)
    try {
      await action()
      if (success) setMessage({ ok: true, text: success })
      await reload()
    } catch (error) {
      setMessage({ ok: false, text: String(error) })
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">{t("platform.members.title")}</h2>
        <p className="mt-1 text-sm text-muted-foreground">{project.name}</p>
        <p className="mt-1 text-xs text-muted-foreground">{t("platform.members.intro")}</p>
      </div>

      <div className="rounded-lg border">
        {members.map((member) => {
          const isMe = member.userId === user?.id
          return (
            <div key={member.userId} className="flex items-center gap-3 border-b px-4 py-2 last:border-b-0">
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium">
                  {member.displayName}
                  {isMe ? ` (${t("platform.members.you")})` : ""}
                </div>
                {member.username && <div className="truncate text-xs text-muted-foreground">{member.username}</div>}
              </div>
              {canManage ? (
                <select
                  aria-label={t("platform.members.role")}
                  value={member.role}
                  onChange={(e) =>
                    void run(() => setProjectMemberRole(project.id, member.userId, e.target.value as ProjectRole))
                  }
                  className="rounded-md border border-input bg-background px-2 py-1 text-sm"
                >
                  {ROLES.map((r) => (
                    <option key={r} value={r}>
                      {t(`platform.roles.${r}`)}
                    </option>
                  ))}
                </select>
              ) : (
                <span className="rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                  {t(`platform.roles.${member.role}`)}
                </span>
              )}
              {(canManage || isMe) && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => void run(() => removeProjectMember(project.id, member.userId))}
                >
                  {isMe ? t("platform.members.leave") : t("platform.members.remove")}
                </Button>
              )}
            </div>
          )
        })}
      </div>

      {canManage ? (
        <div className="max-w-sm space-y-3">
          <div className="space-y-1">
            <Label htmlFor="member-username">{t("platform.members.username")}</Label>
            <Input id="member-username" value={username} onChange={(e) => setUsername(e.target.value)} autoCapitalize="none" />
          </div>
          <div className="space-y-1">
            <Label htmlFor="member-role">{t("platform.members.role")}</Label>
            <select
              id="member-role"
              value={role}
              onChange={(e) => setRole(e.target.value as ProjectRole)}
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
            >
              {ROLES.map((r) => (
                <option key={r} value={r}>
                  {t(`platform.roles.${r}`)}
                </option>
              ))}
            </select>
          </div>
          <Button
            disabled={!username.trim()}
            onClick={() =>
              void run(async () => {
                await addProjectMember(project.id, username.trim(), role)
                setUsername("")
              }, t("platform.members.added"))
            }
          >
            {t("platform.members.add")}
          </Button>
        </div>
      ) : (
        <p className="text-xs text-muted-foreground">{t("platform.members.readOnly")}</p>
      )}

      {message && <p className={`text-sm ${message.ok ? "text-muted-foreground" : "text-destructive"}`}>{message.text}</p>}
    </div>
  )
}
