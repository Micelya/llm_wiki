import { create } from "zustand"

/**
 * Navigation state of the screens of the knowledge layer: which entity,
 * matter or group of doubts is open, and the list's filter. In a store so
 * a notice, a document or a page can send the user straight to it.
 */
interface KnowledgeState {
  /** The entity whose detail is open in the screen of people and organizations. */
  entityId: string | null
  /** The matter whose detail is open in the screen of matters. */
  matterId: string | null
  /** The group of doubts to open first in the screen of doubtful identities. */
  doubtGroup: string | null
  /** The type of entity the list shows (none: all). */
  kind: string | null
  query: string
  openEntity: (id: string | null) => void
  openMatter: (id: string | null) => void
  focusDoubt: (group: string | null) => void
  setKind: (kind: string | null) => void
  setQuery: (query: string) => void
}

export const useKnowledgeStore = create<KnowledgeState>((set) => ({
  entityId: null,
  matterId: null,
  doubtGroup: null,
  kind: null,
  query: "",
  openEntity: (entityId) => set({ entityId }),
  openMatter: (matterId) => set({ matterId }),
  focusDoubt: (doubtGroup) => set({ doubtGroup }),
  setKind: (kind) => set({ kind }),
  setQuery: (query) => set({ query }),
}))
