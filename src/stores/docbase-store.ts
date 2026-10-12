import { create } from "zustand"
import type { DocumentFilter } from "@/commands/docbase"

/**
 * Navigation state of the document base screens: which filter the
 * Documentos screen shows and which document is open. It lives in a
 * store so another screen can send the user to "the documents that
 * could not be read" or straight to one document.
 */
interface DocbaseState {
  filter: DocumentFilter
  openLocationId: number | null
  setFilter: (filter: DocumentFilter) => void
  openDocument: (locationId: number) => void
  closeDocument: () => void
}

export const DEFAULT_DOCUMENT_FILTER: DocumentFilter = { sort: "recent" }

export const useDocbaseStore = create<DocbaseState>((set) => ({
  filter: DEFAULT_DOCUMENT_FILTER,
  openLocationId: null,
  setFilter: (filter) => set({ filter, openLocationId: null }),
  openDocument: (openLocationId) => set({ openLocationId }),
  closeDocument: () => set({ openLocationId: null }),
}))
