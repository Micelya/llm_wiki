import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

/** A folder incorporated into the space. Never copied, never written to. */
export interface Origin {
  id: string
  name: string
  provider: "local-folder"
  location: string
  /** The name of each folder level, outermost first: ["cliente", "caso"]. */
  convention: string[]
  addedAt: number
  lastScanAt: number | null
  /** `null` until the first sweep. `false`: what was known is kept. */
  reachable: boolean | null
}

export interface OriginSummary extends Origin {
  /** Files in the inventory, cloud shortcuts included. */
  files: number
  /** Files still waiting to be read. */
  pending: number
  /** Files that could not be read. */
  errors: number
  /** Cloud shortcuts: detected, not processable. */
  links: number
}

export interface FolderPreview {
  name: string
  /** A few example paths, one folder level per element. */
  examples: string[][]
}

export function listOrigins(): Promise<OriginSummary[]> {
  return invoke<OriginSummary[]>("docbase_list_origins")
}

export function previewFolder(folder: string): Promise<FolderPreview> {
  return invoke<FolderPreview>("docbase_preview_folder", { folder })
}

export function addOrigin(name: string, folder: string, convention: string[]): Promise<Origin> {
  return invoke<Origin>("docbase_add_origin", { name, folder, convention })
}

export function setOriginConvention(originId: string, convention: string[]): Promise<void> {
  return invoke<void>("docbase_set_origin_convention", { originId, convention })
}

export function removeOrigin(originId: string): Promise<void> {
  return invoke<void>("docbase_remove_origin", { originId })
}

/** Asks for a sweep of every origin; results arrive through `onDocbaseChanged`. */
export function rescanOrigins(): Promise<void> {
  return invoke<void>("docbase_rescan")
}

/** Fires whenever something the document base screens show may have changed. */
export function onDocbaseChanged(handler: () => void): Promise<UnlistenFn> {
  return listen("docbase://changed", () => handler())
}

/**
 * Where a document stands, from the inventory to its text.
 * `link`: cloud shortcut, detected and not processable. `unread`: waiting
 * to be read. `unprocessed`: identified, text not extracted yet.
 * `needs-recognition`: its text is only pixels.
 */
export type DocumentState =
  | "link"
  | "unread"
  | "unreadable"
  | "too-large"
  | "unprocessed"
  | "processed"
  | "needs-recognition"
  | "unsupported"
  | "error"

export interface DocumentFilter {
  /** Words to look for in the name, the path and the text. All must be found. */
  query?: string
  originId?: string
  state?: DocumentState | ""
  /** Only documents with more than one location. */
  duplicatesOnly?: boolean
  sort?: "recent" | "name" | "folder"
  limit?: number
  offset?: number
}

/** One location, with what is known of the document it holds. */
export interface DocumentRow {
  locationId: number
  originId: string
  originName: string
  originReachable: boolean | null
  relPath: string
  folder: string
  name: string
  ext: string
  size: number
  mtimeMs: number
  state: DocumentState
  contentId: number | null
  /** Locations holding this same document, this one included. */
  copies: number
  labels: Record<string, string>
}

export interface DocumentPage {
  rows: DocumentRow[]
  /** Rows matching the filter, before `limit` and `offset`. */
  total: number
}

export interface LocationRef {
  locationId: number
  originName: string
  originReachable: boolean | null
  relPath: string
  path: string
  mtimeMs: number
  /** False when it is the same document in another file (another format). */
  sameFile: boolean
}

export interface DocumentDetail extends DocumentRow {
  path: string
  fileHash: string | null
  error: string | null
  wordCount: number | null
  processedAt: number | null
  firstSeenAt: number
  text: string | null
  textEngine: string | null
  /** The folder levels the origin names. */
  labelNames: string[]
  /** Every location of this document, this one first. */
  locations: LocationRef[]
}

export function listDocuments(filter: DocumentFilter): Promise<DocumentPage> {
  return invoke<DocumentPage>("docbase_list_documents", { filter })
}

export function getDocument(locationId: number): Promise<DocumentDetail | null> {
  return invoke<DocumentDetail | null>("docbase_get_document", { locationId })
}

/** An empty value undoes the correction: the label is again what the folder says. */
export function setDocumentLabel(locationId: number, name: string, value: string): Promise<void> {
  return invoke<void>("docbase_set_label", { locationId, name, value })
}

/** Opens the original with the system's program, or shows it in its folder. */
export function openLocation(locationId: number, reveal = false): Promise<void> {
  return invoke<void>("docbase_open_location", { locationId, reveal })
}
