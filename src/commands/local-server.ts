import { invoke } from "@tauri-apps/api/core"

/**
 * The app's one server inside the machine (D-495, `src-tauri/src/local_server`):
 * the web clipper, the local API and the MCP, at http://127.0.0.1:19827.
 * Other programs use the base with their own token; by default they only
 * read, and what they ask to change waits in Atención for a person.
 */

export type LocalServerState = "starting" | "running" | "port-in-use" | "stopped"

export interface LocalServerStatus {
  state: LocalServerState
  /** `http://127.0.0.1:19827`. */
  address: string
  /** Where an assistant connects by MCP. */
  mcpAddress: string
  /** Whether programs may use it (the web clipper always may). */
  programsEnabled: boolean
}

/** `read`: only the readings (the default); `change`: also ask for changes, which wait in Atención. */
export type ProgramPermission = "read" | "change"

export interface LocalProgram {
  id: string
  name: string
  permission: ProgramPermission
  createdAt: number
  lastUsedAt: number | null
  /** How many operations it used in the last 30 days. */
  uses: number
}

/** A program just added or with its token renewed: the token, to copy into it. */
export interface ProgramWithToken {
  program: LocalProgram
  token: string
  /** Whether the system keychain kept it, to copy it again later. */
  kept: boolean
}

export function getLocalServerStatus(): Promise<LocalServerStatus> {
  return invoke<LocalServerStatus>("local_server_status")
}

/** Turns the programs on or off; the web clipper keeps working. */
export function setProgramsEnabled(enabled: boolean): Promise<LocalServerStatus> {
  return invoke<LocalServerStatus>("local_server_set_enabled", { enabled })
}

export function listPrograms(): Promise<LocalProgram[]> {
  return invoke<LocalProgram[]>("local_programs")
}

export function addProgram(name: string, permission: ProgramPermission): Promise<ProgramWithToken> {
  return invoke<ProgramWithToken>("local_program_add", { name, permission })
}

/** The token of a program, from the system keychain. */
export function programToken(id: string): Promise<string> {
  return invoke<string>("local_program_token", { id })
}

/** A new token: the old one stops working at once. */
export function renewProgram(id: string): Promise<ProgramWithToken> {
  return invoke<ProgramWithToken>("local_program_renew", { id })
}

export function setProgramPermission(id: string, permission: ProgramPermission): Promise<LocalProgram> {
  return invoke<LocalProgram>("local_program_set_permission", { id, permission })
}

/** Removes a program: its token stops working; what it asked for stays in the record. */
export function removeProgram(id: string): Promise<void> {
  return invoke("local_program_remove", { id })
}
