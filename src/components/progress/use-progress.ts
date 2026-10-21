import { useEffect } from "react"
import { getProgress, type Progress, type ProgressScope } from "@/commands/docbase"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { useProgressStore } from "@/stores/progress-store"

/** How often the figures are asked again while they are on screen. */
export const PROGRESS_EVERY_MS = 3_000

/** Whether a scope is the whole archive. */
export function isWholeArchive(scope: ProgressScope | null | undefined): boolean {
  return !scope?.originId
}

/** The key of the query of a scope: one per origin and folder. */
export function progressKey(scope: ProgressScope | null | undefined): string {
  return isWholeArchive(scope) ? "progress" : `progress:${scope?.originId}/${scope?.folder ?? ""}`
}

/**
 * Asks the base for the figures again every few seconds and whenever it
 * says something changed, while the component is mounted.
 */
export function usePolledProgress(scope: ProgressScope | null, enabled = true): { data: Progress | null; error: string | null; reload: () => void } {
  const whole = isWholeArchive(scope)
  const query = useDocbaseQuery(progressKey(scope), () => (enabled ? getProgress(whole ? null : scope) : Promise.resolve(null)))
  const { reload } = query
  useEffect(() => {
    if (!enabled) return
    const timer = setInterval(() => {
      // Nothing to show while the window is hidden.
      if (typeof document === "undefined" || document.visibilityState !== "hidden") reload()
    }, PROGRESS_EVERY_MS)
    return () => clearInterval(timer)
  }, [enabled, reload])
  return { data: query.data, error: query.error, reload }
}

/**
 * Mounted once with the layout: keeps the figures of the whole archive in
 * the progress store for the line at the foot, Inicio and «En curso».
 */
export function ProgressPoller() {
  const { data, error, reload } = usePolledProgress(null)
  const set = useProgressStore((s) => s.set)
  const setRefresh = useProgressStore((s) => s.setRefresh)
  useEffect(() => {
    set(data, error)
  }, [data, error, set])
  useEffect(() => {
    setRefresh(reload)
  }, [reload, setRefresh])
  return null
}

/**
 * The figures of a scope: those of the store for the whole archive, or
 * asked on their own for an origin or a folder while they are on screen.
 */
export function useProgress(scope: ProgressScope | null): { progress: Progress | null; error: string | null; refresh: () => void } {
  const whole = isWholeArchive(scope)
  const stored = useProgressStore((s) => s.progress)
  const storedError = useProgressStore((s) => s.error)
  const refresh = useProgressStore((s) => s.refresh)
  const own = usePolledProgress(scope, !whole)
  return whole
    ? { progress: stored, error: storedError, refresh }
    : {
        progress: own.data,
        error: own.error,
        refresh: () => {
          own.reload()
          refresh()
        },
      }
}
