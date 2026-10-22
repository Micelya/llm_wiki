import { create } from "zustand"

/**
 * Whether the articles show a floating card when the mouse rests on, or
 * the keyboard reaches, a numbered note or a link (P-246). On unless the
 * person turned it off in Settings → Interface; kept in `app-state.json`.
 */
export interface PreviewCardsState {
  enabled: boolean
  setEnabled: (enabled: boolean) => void
}

export const usePreviewCardsStore = create<PreviewCardsState>((set) => ({
  enabled: true,
  setEnabled: (enabled) => set({ enabled }),
}))
