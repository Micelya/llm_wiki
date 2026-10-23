import { invoke } from "@tauri-apps/api/core"

/**
 * Micelya's work folder (D-450): where the app keeps what it brings by
 * itself, the web clips («Recortes web») and the records of the portals
 * («Expedientes»). It is an origin of the base (`src-tauri/src/work_folder.rs`).
 */
export interface WorkFolderStatus {
  path: string
  clips: string
  records: string
  /** «Generados»: the documents generated from a template. */
  generated: string
  /** The origin of the base that holds it. */
  originName: string | null
  /** That origin is the folder itself, not one that holds it. */
  originOwn: boolean
  /** Why it is not ready, in words. */
  error: string | null
}

/** The work folder; asking also makes sure it exists and is an origin. */
export function getWorkFolder(): Promise<WorkFolderStatus> {
  return invoke<WorkFolderStatus>("work_folder_status")
}

/** Changes it; it is kept only if the base can take it as an origin. */
export function setWorkFolder(path: string): Promise<WorkFolderStatus> {
  return invoke<WorkFolderStatus>("work_folder_set", { path })
}

/** Opens the work folder, or its clips, records or generated documents. */
export function openWorkFolder(which?: "clips" | "records" | "generated"): Promise<void> {
  return invoke("work_folder_open", { which: which ?? null })
}

export interface NewFile {
  locationId: number
  name: string
  folder: string
  originName: string
  firstSeenAt: number
}

export interface NewFiles {
  total: number
  files: NewFile[]
}

/** The clips the base inventoried since the person last saw them. */
export function listNewClips(): Promise<NewFiles> {
  return invoke<NewFiles>("work_folder_new_clips")
}

/** The person saw the new clips: they are no longer new. */
export function markClipsSeen(): Promise<void> {
  return invoke("work_folder_clips_seen")
}

/** How the web clipper's local server is. */
export type ClipServerState = "starting" | "running" | "port-in-use" | "stopped"

export function getClipServerState(): Promise<ClipServerState> {
  return invoke<ClipServerState>("clip_server_status")
}
