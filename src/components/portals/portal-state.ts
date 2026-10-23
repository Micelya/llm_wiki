import type { PortalStatus } from "@/commands/portals"

/**
 * Whether a walk runs over the pages of a record and only the app works
 * on it: it already reached the record (before, the person sees the app
 * fill the portal's form), the portal needs no person (no challenge, no
 * signing in), it is not processing a query and the person did not take
 * the window.
 */
export function downloadingAlone(status: PortalStatus | null): boolean {
  if (!status?.running || status.manual) return false
  const p = status.progress
  return p.state === "working" && !p.busy && p.page !== null
}

/**
 * Whether the portal's window shows over its area. While the app works
 * alone it is hidden and the area shows the progress with room; it shows
 * again when the portal needs the person (its challenge, signing in),
 * while it processes a query (so its own «Cancelar» is at hand), when the
 * person uses it, or when she asks to see it («Ver el portal»).
 */
export function showsPortal(status: PortalStatus | null, asked: boolean): boolean {
  return asked || !downloadingAlone(status)
}

/** A figure of the progress: what it counts and how many. */
export interface ProgressFigure {
  label: string
  value: number
}

/**
 * The figures of the walk in course, in the order they are read: the
 * page, the entries seen, the documents, what was downloaded, what was
 * already there, what is left of the page and what failed.
 */
export function progressFigures(status: PortalStatus): ProgressFigure[] {
  const p = status.progress
  const figures: ProgressFigure[] = []
  if (p.page) figures.push({ label: "Página", value: p.page })
  figures.push({ label: "Actuaciones vistas", value: p.seen })
  figures.push({ label: "Con documento", value: p.withDocument })
  figures.push({ label: "Bajados ahora", value: p.downloaded })
  figures.push({ label: "Ya estaban", value: p.already })
  figures.push({ label: "Faltan de esta página", value: p.pending })
  if (p.failed > 0) figures.push({ label: "No se pudieron bajar", value: p.failed })
  return figures
}
