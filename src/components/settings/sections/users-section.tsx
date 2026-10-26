import { useCallback, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { usePlatformInfo } from "@/hooks/use-platform-info"
import {
  createUser,
  listUsers,
  setUserAdmin,
  setUserDisabled,
  setUserPassword,
  type ServerUser,
} from "@/lib/platform-api"

export function UsersSection() {
  const { t } = useTranslation()
  const { user: me } = usePlatformInfo()
  const [users, setUsers] = useState<ServerUser[]>([])
  const [username, setUsername] = useState("")
  const [displayName, setDisplayName] = useState("")
  const [password, setPassword] = useState("")
  const [admin, setAdmin] = useState(false)
  const [resetting, setResetting] = useState<string | null>(null)
  const [newPassword, setNewPassword] = useState("")
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null)

  const reload = useCallback(async () => {
    try {
      setUsers(await listUsers())
    } catch (error) {
      setMessage({ ok: false, text: String(error) })
    }
  }, [])

  useEffect(() => {
    void reload()
  }, [reload])

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
        <h2 className="text-xl font-semibold">{t("platform.users.title")}</h2>
        <p className="mt-1 text-sm text-muted-foreground">{t("platform.users.intro")}</p>
      </div>

      <div className="rounded-lg border">
        {users.map((u) => (
          <div key={u.id} className="border-b px-4 py-2 last:border-b-0">
            <div className="flex flex-wrap items-center gap-2">
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium">
                  {u.displayName}
                  {u.isPlatformAdmin && (
                    <span className="ml-2 rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                      {t("platform.users.admin")}
                    </span>
                  )}
                  {u.disabled && (
                    <span className="ml-2 rounded bg-destructive/10 px-1.5 py-0.5 text-xs text-destructive">
                      {t("platform.users.disabled")}
                    </span>
                  )}
                </div>
                <div className="truncate text-xs text-muted-foreground">{u.username}</div>
              </div>
              <Button variant="outline" size="sm" onClick={() => setResetting(resetting === u.id ? null : u.id)}>
                {t("platform.users.setPassword")}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void run(() => setUserAdmin(u.id, !u.isPlatformAdmin))}
              >
                {u.isPlatformAdmin ? t("platform.users.demote") : t("platform.users.promote")}
              </Button>
              {u.id !== me?.id && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => void run(() => setUserDisabled(u.id, !u.disabled))}
                >
                  {u.disabled ? t("platform.users.enable") : t("platform.users.disable")}
                </Button>
              )}
            </div>
            {resetting === u.id && (
              <div className="mt-2 flex max-w-sm gap-2">
                <Input
                  type="password"
                  autoComplete="new-password"
                  placeholder={t("platform.users.newPassword")}
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                />
                <Button
                  size="sm"
                  disabled={!newPassword}
                  onClick={() =>
                    void run(async () => {
                      await setUserPassword(u.id, newPassword)
                      setNewPassword("")
                      setResetting(null)
                    })
                  }
                >
                  {t("platform.users.setPassword")}
                </Button>
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="max-w-sm space-y-3">
        <h3 className="text-sm font-medium">{t("platform.users.create")}</h3>
        <div className="space-y-1">
          <Label htmlFor="new-username">{t("platform.users.username")}</Label>
          <Input id="new-username" value={username} onChange={(e) => setUsername(e.target.value)} autoCapitalize="none" />
        </div>
        <div className="space-y-1">
          <Label htmlFor="new-display-name">{t("platform.users.displayName")}</Label>
          <Input id="new-display-name" value={displayName} onChange={(e) => setDisplayName(e.target.value)} />
        </div>
        <div className="space-y-1">
          <Label htmlFor="new-user-password">{t("platform.users.password")}</Label>
          <Input id="new-user-password" type="password" autoComplete="new-password" value={password} onChange={(e) => setPassword(e.target.value)} />
        </div>
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={admin} onChange={(e) => setAdmin(e.target.checked)} />
          {t("platform.users.makeAdmin")}
        </label>
        <Button
          disabled={!username.trim() || !password}
          onClick={() =>
            void run(async () => {
              await createUser({
                username: username.trim(),
                displayName: displayName.trim() || username.trim(),
                password,
                isPlatformAdmin: admin,
              })
              setUsername("")
              setDisplayName("")
              setPassword("")
              setAdmin(false)
            }, t("platform.users.created"))
          }
        >
          {t("platform.users.create")}
        </Button>
      </div>

      {message && <p className={`text-sm ${message.ok ? "text-muted-foreground" : "text-destructive"}`}>{message.text}</p>}
    </div>
  )
}
