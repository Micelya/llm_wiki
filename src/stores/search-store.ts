import { create } from "zustand"

/**
 * What the search of the top bar asks (D-408): the search screen runs it
 * when it opens and every time it is asked again, the same words too.
 */
interface SearchState {
  query: string
  /** Grows with each ask, so the same words asked again search again. */
  asked: number
  ask: (query: string) => void
}

export const useSearchStore = create<SearchState>((set) => ({
  query: "",
  asked: 0,
  ask: (query) => set((state) => ({ query, asked: state.asked + 1 })),
}))
