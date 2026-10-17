import { create } from "zustand"
import type { AttentionNotice } from "@/lib/attention/types"

/**
 * What each source raises right now, by source id. A source publishes
 * its whole list every time it changes: what it leaves out is gone. The
 * attention view, the navigation counter and Inicio read from here.
 */
interface AttentionState {
  bySource: Record<string, AttentionNotice[]>
  publish: (source: string, notices: AttentionNotice[]) => void
  clear: (source: string) => void
}

export const useAttentionStore = create<AttentionState>((set) => ({
  bySource: {},
  publish: (source, notices) =>
    set((state) => (sameNotices(state.bySource[source], notices) ? state : { bySource: { ...state.bySource, [source]: notices } })),
  clear: (source) =>
    set((state) => {
      if (!(source in state.bySource)) return state
      const next = { ...state.bySource }
      delete next[source]
      return { bySource: next }
    }),
}))

/**
 * Two lists say the same when every line has the same id, numbers and
 * words; functions (actions) are rebuilt on every render and do not count.
 * It keeps a source that polls from re-rendering the view each time.
 */
function sameNotices(a: AttentionNotice[] | undefined, b: AttentionNotice[]): boolean {
  if (!a || a.length !== b.length) return false
  return a.every((notice, index) => fingerprint(notice) === fingerprint(b[index]))
}

function fingerprint(notice: AttentionNotice): string {
  return JSON.stringify(notice, (_key, value) => (typeof value === "function" ? undefined : value))
}
