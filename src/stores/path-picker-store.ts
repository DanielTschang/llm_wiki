/**
 * Requests for the web build's file/folder picker (ServerPathPickerHost).
 * The browser cannot use the OS dialog for paths on the server, so the
 * web platform's `openDialog` / `saveDialog` route through here and resolve
 * with server paths, exactly like the Tauri dialogs on desktop.
 */
import { create } from "zustand"
import type { OpenDialogOptions, SaveDialogOptions } from "@/platform/types"

export type PathPickerRequest =
  | ({ mode: "open" } & OpenDialogOptions)
  | ({ mode: "save" } & SaveDialogOptions)

export type PathPickerResult = string | string[] | null

interface PendingPick {
  id: number
  request: PathPickerRequest
  resolve: (result: PathPickerResult) => void
}

interface PathPickerState {
  current: PendingPick | null
  queue: PendingPick[]
  settle: (id: number, result: PathPickerResult) => void
}

export const usePathPickerStore = create<PathPickerState>((set, get) => ({
  current: null,
  queue: [],
  settle: (id, result) => {
    const { current, queue } = get()
    if (!current || current.id !== id) return
    const [next, ...rest] = queue
    set({ current: next ?? null, queue: rest })
    current.resolve(result)
  },
}))

let nextPickId = 1

export function requestServerPath(request: PathPickerRequest): Promise<PathPickerResult> {
  return new Promise((resolve) => {
    const pick: PendingPick = { id: nextPickId++, request, resolve }
    const state = usePathPickerStore.getState()
    if (state.current) usePathPickerStore.setState({ queue: [...state.queue, pick] })
    else usePathPickerStore.setState({ current: pick })
  })
}
