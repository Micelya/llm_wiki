import { create } from "zustand"
import type { PortalStatus } from "@/commands/portals"

/** How a walk over a record ends. */
export const ENDED_STATES = ["done", "stopped", "error"]

/**
 * How the walk over the portal goes, for the whole app: «Portales», the
 * menu, the line at the foot of the window and Atención read it from here.
 * A walk goes on while the person is in other screens (the portal's window
 * is only hidden, D-461); when it ends there, its end stays told in the
 * foot and in Atención until she opens «Portales».
 */
interface PortalState {
  status: PortalStatus | null
  /** The last walk ended while «Portales» was not on screen, and she has not been there since. */
  endUnseen: boolean
  /** «Portales» is on screen. */
  open: boolean
  publish: (status: PortalStatus) => void
  setOpen: (open: boolean) => void
}

export const usePortalStore = create<PortalState>((set, get) => ({
  status: null,
  endUnseen: false,
  open: false,
  publish: (status) => {
    const before = get()
    const ended = Boolean(before.status?.running) && !status.running && ENDED_STATES.includes(status.progress.state)
    // A walk in course is told by itself; its end, until it is seen.
    const endUnseen = status.running ? false : ended ? !before.open : before.endUnseen
    set({ status, endUnseen })
  },
  setOpen: (open) => set(open ? { open, endUnseen: false } : { open }),
}))
