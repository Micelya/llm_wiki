import { create } from "zustand"
import type { Progress } from "@/commands/docbase"

/**
 * How far the processing of the whole archive went, stage by stage
 * (D-424), as the poller of the layout last read it: the line at the foot
 * of the window, the tab «En curso» and the figure of Inicio read it from
 * here, so the base is asked once however many of them are on screen.
 */
interface ProgressState {
  progress: Progress | null
  error: string | null
  /** Asks the base again now (after pausing or turning on the reading). */
  refresh: () => void
  set: (progress: Progress | null, error: string | null) => void
  setRefresh: (refresh: () => void) => void
}

export const useProgressStore = create<ProgressState>((set) => ({
  progress: null,
  error: null,
  refresh: () => {},
  set: (progress, error) => set((state) => (error ? { error } : { progress: progress ?? state.progress, error: null })),
  setRefresh: (refresh) => set({ refresh }),
}))
