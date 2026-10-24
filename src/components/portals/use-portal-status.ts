import { useEffect } from "react"
import { getPortalStatus, onPortalStatus, type PortalStatus } from "@/commands/portals"
import { usePortalStore } from "@/stores/portal-store"

// One listener for the whole app: the page, the navigation, the foot of
// the window and Atención follow the same status (`portal-store`).
let started = false

/** Tells every screen the status a command answered with. */
export function publishPortalStatus(status: PortalStatus): void {
  usePortalStore.getState().publish(status)
}

function start(): void {
  if (started) return
  started = true
  try {
    getPortalStatus().then(publishPortalStatus).catch(() => {})
    onPortalStatus(publishPortalStatus).catch(() => {})
  } catch {
    // Outside the app (tests): there is no portal.
  }
}

/** How the walk over the portal goes, as the runner tells it. */
export function usePortalStatus(): PortalStatus | null {
  useEffect(() => {
    start()
  }, [])
  return usePortalStore((s) => s.status)
}
