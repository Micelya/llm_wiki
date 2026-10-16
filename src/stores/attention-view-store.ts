import { create } from "zustand"

/**
 * Navigation state of the screens the attention view leads to: which
 * family of similar documents to open. In a store so a notice can send
 * the user straight to it.
 */
interface AttentionViewState {
  /** The principal (content id) of the family to open in the similar documents screen. */
  family: number | null
  focusFamily: (principal: number | null) => void
}

export const useAttentionViewStore = create<AttentionViewState>((set) => ({
  family: null,
  focusFamily: (family) => set({ family }),
}))
