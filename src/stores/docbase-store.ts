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
  /** A document the open file includes, when that is what is open (D-560). */
  openVersionId: number | null
  setFilter: (filter: DocumentFilter) => void
  /** Opens a location; with `versionId`, one of the documents its file includes. */
  openDocument: (locationId: number, versionId?: number | null) => void
  closeDocument: () => void
}

export const DEFAULT_DOCUMENT_FILTER: DocumentFilter = { sort: "recent" }

export const useDocbaseStore = create<DocbaseState>((set) => ({
  filter: DEFAULT_DOCUMENT_FILTER,
  openLocationId: null,
  openVersionId: null,
  setFilter: (filter) => set({ filter, openLocationId: null, openVersionId: null }),
  openDocument: (openLocationId, versionId = null) => set({ openLocationId, openVersionId: versionId }),
  closeDocument: () => set({ openLocationId: null, openVersionId: null }),
}))
