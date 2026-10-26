import { beforeEach, describe, expect, it, vi } from "vitest"

const invoke = vi.fn()
vi.mock("@/platform", () => ({ invoke: (...args: unknown[]) => invoke(...args) }))

async function load() {
  vi.resetModules()
  return import("@/lib/platform-api")
}

beforeEach(() => {
  invoke.mockReset()
})

describe("getWhoAmI", () => {
  it("reports a server without accounts as null and remembers that", async () => {
    invoke.mockRejectedValue("Unknown command: whoami")
    const api = await load()
    expect(await api.getWhoAmI()).toBeNull()
    expect(await api.getWhoAmI()).toBeNull()
    expect(invoke).toHaveBeenCalledTimes(1)
  })

  it("asks again after a failure that is not 'unknown command'", async () => {
    invoke.mockRejectedValueOnce("fetch failed").mockResolvedValueOnce({
      kind: "user",
      user: { id: "u1", displayName: "Ann", isPlatformAdmin: false },
    })
    const api = await load()
    expect(await api.getWhoAmI()).toBeNull()
    const who = await api.getWhoAmI()
    expect(who?.kind).toBe("user")
    expect(invoke).toHaveBeenCalledTimes(2)
  })

  it("keeps the answer for a signed-in user", async () => {
    invoke.mockResolvedValue({ kind: "user", user: { id: "u1", displayName: "Ann", isPlatformAdmin: true } })
    const api = await load()
    const who = await api.getWhoAmI()
    await api.getWhoAmI()
    expect(invoke).toHaveBeenCalledTimes(1)
    expect(api.isPlatformAdmin(who)).toBe(true)
    expect(api.currentUser(who)?.displayName).toBe("Ann")
  })
})

describe("roles of the signed-in principal", () => {
  it("treats the operator as an administrator and a user by their flag", async () => {
    const api = await load()
    expect(api.isPlatformAdmin(null)).toBe(false)
    expect(api.isPlatformAdmin({ kind: "operator", isPlatformAdmin: true })).toBe(true)
    expect(api.isPlatformAdmin({ kind: "worker", isPlatformAdmin: false })).toBe(false)
    const user = { id: "u", username: "a", displayName: "A", email: null, isPlatformAdmin: false, disabled: false, isService: false }
    expect(api.isPlatformAdmin({ kind: "user", user })).toBe(false)
    expect(api.currentUser({ kind: "operator", isPlatformAdmin: true })).toBeNull()
  })
})

describe("the server calls", () => {
  it("send the arguments the server expects", async () => {
    invoke.mockResolvedValue(undefined)
    const api = await load()
    await api.addProjectMember("p1", "ann", "editor")
    await api.setProjectMemberRole("p1", "u1", "viewer")
    await api.removeProjectMember("p1", "u1")
    await api.changeMyPassword("old", "new")
    await api.setUserDisabled("u1", true)
    expect(invoke.mock.calls).toEqual([
      ["add_project_member", { projectId: "p1", username: "ann", role: "editor" }],
      ["set_project_member_role", { projectId: "p1", userId: "u1", role: "viewer" }],
      ["remove_project_member", { projectId: "p1", userId: "u1" }],
      ["change_my_password", { currentPassword: "old", newPassword: "new" }],
      ["set_user_disabled", { userId: "u1", disabled: true }],
    ])
  })
})
