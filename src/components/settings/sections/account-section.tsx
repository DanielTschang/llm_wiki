import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { usePlatformInfo } from "@/hooks/use-platform-info"
import { changeMyPassword, signOut } from "@/lib/platform-api"

export function AccountSection() {
  const { t } = useTranslation()
  const { user, isAdmin } = usePlatformInfo()
  const [current, setCurrent] = useState("")
  const [next, setNext] = useState("")
  const [again, setAgain] = useState("")
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null)
  const [busy, setBusy] = useState(false)

  async function handleChange() {
    if (next !== again) {
      setMessage({ ok: false, text: t("platform.account.passwordMismatch") })
      return
    }
    setBusy(true)
    setMessage(null)
    try {
      await changeMyPassword(current, next)
      setCurrent("")
      setNext("")
      setAgain("")
      setMessage({ ok: true, text: t("platform.account.passwordChanged") })
    } catch (error) {
      setMessage({ ok: false, text: String(error) })
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">{t("platform.account.title")}</h2>
        {user && (
          <p className="mt-1 text-sm text-muted-foreground">
            {t("platform.account.signedInAs", { name: user.displayName })}
            {user.username ? ` (${user.username})` : ""}
            {isAdmin ? ` · ${t("platform.account.admin")}` : ""}
          </p>
        )}
      </div>

      <div className="max-w-sm space-y-3">
        <h3 className="text-sm font-medium">{t("platform.account.changePassword")}</h3>
        <div className="space-y-1">
          <Label htmlFor="current-password">{t("platform.account.currentPassword")}</Label>
          <Input id="current-password" type="password" autoComplete="current-password" value={current} onChange={(e) => setCurrent(e.target.value)} />
        </div>
        <div className="space-y-1">
          <Label htmlFor="new-password">{t("platform.account.newPassword")}</Label>
          <Input id="new-password" type="password" autoComplete="new-password" value={next} onChange={(e) => setNext(e.target.value)} />
        </div>
        <div className="space-y-1">
          <Label htmlFor="again-password">{t("platform.account.confirmPassword")}</Label>
          <Input id="again-password" type="password" autoComplete="new-password" value={again} onChange={(e) => setAgain(e.target.value)} />
        </div>
        {message && (
          <p className={`text-sm ${message.ok ? "text-muted-foreground" : "text-destructive"}`}>{message.text}</p>
        )}
        <Button onClick={handleChange} disabled={busy || !current || !next || !again}>
          {t("platform.account.changePassword")}
        </Button>
      </div>

      <div>
        <Button variant="outline" onClick={() => void signOut()}>
          {t("platform.account.signOut")}
        </Button>
      </div>
    </div>
  )
}
