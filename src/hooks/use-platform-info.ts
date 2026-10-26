import { useEffect, useState } from "react"
import { currentUser, getWhoAmI, isPlatformAdmin, type ServerUser, type WhoAmI } from "@/lib/platform-api"

export interface PlatformInfo {
  /** Still asking the server. */
  loading: boolean
  /** The server has accounts and projects of its own. */
  isPlatform: boolean
  isAdmin: boolean
  user: ServerUser | null
  who: WhoAmI | null
}

/** Whether this is a platform server, and who is signed in on it. */
export function usePlatformInfo(): PlatformInfo {
  const [state, setState] = useState<{ loading: boolean; who: WhoAmI | null }>({
    loading: true,
    who: null,
  })
  useEffect(() => {
    let cancelled = false
    getWhoAmI().then((who) => {
      if (!cancelled) setState({ loading: false, who })
    })
    return () => {
      cancelled = true
    }
  }, [])
  return {
    loading: state.loading,
    isPlatform: state.who !== null,
    isAdmin: isPlatformAdmin(state.who),
    user: currentUser(state.who),
    who: state.who,
  }
}
