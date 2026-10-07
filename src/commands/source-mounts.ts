import { invoke } from "@tauri-apps/api/core"

/** A folder that appears under `raw/sources/<name>` without being copied. */
export interface SourceMount {
  id: string
  name: string
  provider: "localFolder"
  location: string
}

export async function listSourceMounts(projectPath: string): Promise<SourceMount[]> {
  return invoke<SourceMount[]>("list_source_mounts", { projectPath })
}

export async function addSourceMount(
  projectPath: string,
  name: string,
  folder: string,
): Promise<SourceMount> {
  return invoke<SourceMount>("add_source_mount", { projectPath, name, folder })
}
