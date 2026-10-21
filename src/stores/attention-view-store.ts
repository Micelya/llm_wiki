import { create } from "zustand"
import type { ProgressScope } from "@/commands/docbase"

/** The tabs of the attention view (D-424). */
export type AttentionTab = "problem" | "decision" | "ongoing"

/**
 * Navigation state of the attention view and of the screens it leads to:
 * which tab it shows, what part of the archive «En curso» is about, and
 * which family of similar documents to open. In a store so a notice, the
 * line at the foot of the window or Inicio can send the user straight to
 * it.
 */
interface AttentionViewState {
  /** The tab the person chose; `null` until she chooses (the view picks the first with something). */
  tab: AttentionTab | null
  setTab: (tab: AttentionTab | null) => void
  /** The origin or folder «En curso» is narrowed to; empty, the whole archive. */
  scope: ProgressScope
  setScope: (scope: ProgressScope) => void
  /** The principal (content id) of the family to open in the similar documents screen. */
  family: number | null
  focusFamily: (principal: number | null) => void
}

export const useAttentionViewStore = create<AttentionViewState>((set) => ({
  tab: null,
  setTab: (tab) => set({ tab }),
  scope: {},
  setScope: (scope) => set({ scope }),
  family: null,
  focusFamily: (family) => set({ family }),
}))
