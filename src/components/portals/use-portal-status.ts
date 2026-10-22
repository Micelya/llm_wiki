import { useEffect, useState } from "react"
import { getPortalStatus, onPortalStatus, type PortalStatus } from "@/commands/portals"

// One listener for the whole app: the page and the navigation follow the
// same status.
let current: PortalStatus | null = null
let started = false
const subscribers = new Set<(status: PortalStatus) => void>()

/** Tells every screen the status a command answered with. */
export function publishPortalStatus(status: PortalStatus): void {
  current = status
  for (const subscriber of subscribers) subscriber(status)
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
  const [status, setStatus] = useState<PortalStatus | null>(current)
  useEffect(() => {
    start()
    subscribers.add(setStatus)
    if (current) setStatus(current)
    return () => {
      subscribers.delete(setStatus)
    }
  }, [])
  return status
}
