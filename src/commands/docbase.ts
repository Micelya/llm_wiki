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
