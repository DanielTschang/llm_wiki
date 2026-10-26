/**
 * The platform server's accounts and projects, as the browser sees them.
 *
 * A plain self-hosted server has no accounts: it does not know `whoami`, and
 * everything here reports "not a platform server". On a platform server every
 * request is made by a signed-in user, and what they may do in a project is
 * their role in it.
 */
import { invoke } from "@/platform"

export type ProjectRole = "owner" | "editor" | "viewer"

export interface ServerProject {
  id: string
  name: string
  /** `/projects/<id>`: where the project's files are addressed. */
  path: string
  /** The caller's role; null for an administrator who is not a member. */
  role: ProjectRole | null
  memberCount: number
  createdAt: string
}

export interface ServerUser {
  id: string
  username: string | null
  displayName: string
  email: string | null
  isPlatformAdmin: boolean
  disabled: boolean
  isService: boolean
}

export interface ProjectMember {
  userId: string
  username: string | null
  displayName: string
  role: ProjectRole
  addedAt: string
}

export type WhoAmI =
  | { kind: "user"; user: ServerUser }
  | { kind: "operator" | "worker"; isPlatformAdmin: boolean }

let whoAmI: Promise<WhoAmI | null> | null = null

/** Who is signed in, or null when the server has no accounts. */
export function getWhoAmI(): Promise<WhoAmI | null> {
  if (!whoAmI) {
    whoAmI = invoke<WhoAmI>("whoami").catch((error: unknown) => {
      // A server without accounts does not know the command. Any other
      // failure (network, a restart) is worth asking again about.
      if (!String(error).includes("Unknown command")) whoAmI = null
      return null
    })
  }
  return whoAmI
}

export function forgetWhoAmI(): void {
  whoAmI = null
}

export function isPlatformAdmin(who: WhoAmI | null): boolean {
  if (!who) return false
  return who.kind === "user" ? who.user.isPlatformAdmin : who.isPlatformAdmin
}

export function currentUser(who: WhoAmI | null): ServerUser | null {
  return who?.kind === "user" ? who.user : null
}

export const listMyProjects = () => invoke<ServerProject[]>("list_projects")

export const listProjectMembers = (projectId: string) =>
  invoke<ProjectMember[]>("list_project_members", { projectId })

export const addProjectMember = (projectId: string, username: string, role: ProjectRole) =>
  invoke<void>("add_project_member", { projectId, username, role })

export const setProjectMemberRole = (projectId: string, userId: string, role: ProjectRole) =>
  invoke<void>("set_project_member_role", { projectId, userId, role })

export const removeProjectMember = (projectId: string, userId: string) =>
  invoke<void>("remove_project_member", { projectId, userId })

export const lookupUser = (username: string) =>
  invoke<{ id: string; username: string; displayName: string } | null>("lookup_user", { username })

export const listUsers = () => invoke<ServerUser[]>("list_users")

export const createUser = (input: {
  username: string
  displayName: string
  password: string
  isPlatformAdmin: boolean
}) => invoke<ServerUser>("create_user", input)

export const setUserPassword = (userId: string, password: string) =>
  invoke<void>("set_user_password", { userId, password })

export const setUserDisabled = (userId: string, disabled: boolean) =>
  invoke<void>("set_user_disabled", { userId, disabled })

export const setUserAdmin = (userId: string, admin: boolean) =>
  invoke<void>("set_user_admin", { userId, admin })

export const changeMyPassword = (currentPassword: string, newPassword: string) =>
  invoke<void>("change_my_password", { currentPassword, newPassword })

/** Ends the session and returns to the sign-in page. */
export async function signOut(): Promise<void> {
  const base = import.meta.env.VITE_LLM_WIKI_SERVER_URL ?? ""
  try {
    await fetch(`${base}/logout`, {
      method: "POST",
      credentials: "include",
      headers: { "X-LLM-Wiki-Client": "web" },
    })
  } finally {
    forgetWhoAmI()
    window.location.assign(`${base}/login`)
  }
}
