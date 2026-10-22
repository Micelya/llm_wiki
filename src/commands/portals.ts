import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

/** The only portal with a connector today: the PJN's Consulta Web. */
export const PJN = "pjn"

/** How the walk over a record goes. */
export interface PortalProgress {
  /** `working`, `needs-person`, `done`, `stopped`, `error`, or empty before the first one. */
  state: string
  message: string
  reference: string | null
  /** What the portal asks a person: `challenge` or `login`. */
  person: string | null
  /** The portal is processing a query: the app waits and the window is free for its «Cancelar». */
  busy: boolean
  page: number | null
  seen: number
  withDocument: number
  downloaded: number
  already: number
  failed: number
  warnings: string[]
}

export interface PortalStatus {
  /** The record being brought, as people write it. */
  running: string | null
  queue: string[]
  /** The person uses the portal's window by themselves. */
  manual: boolean
  progress: PortalProgress
}

export interface FollowedLink {
  reference: string
  jurisdiction: string
  number: number
  year: number
  detail: string | null
  followed: boolean
}

/** A record followed: brought once, updated on request. */
export interface FollowedRecord {
  folder: string
  reference: string
  jurisdiction: string
  number: number
  year: number
  title: string | null
  office: string | null
  situation: string | null
  followedAt: string
  updatedAt: string | null
  lastResult: string | null
  entries: number
  withDocument: number
  downloaded: number
  already: number
  failed: number
  pending: number
  matterId: string | null
  linked: FollowedLink[]
}

export interface FollowedList {
  name: string
  /** Where the portal's records are kept. */
  folder: string
  records: FollowedRecord[]
}

export interface Jurisdiction {
  code: string
  name: string
}

/** Shows the portal's window over this area of the app's window (CSS pixels). */
export function showPortal(portal: string, rect: { x: number; y: number; width: number; height: number }): Promise<void> {
  return invoke<void>("portal_show", { portal, ...rect })
}

export function hidePortal(portal: string): Promise<void> {
  return invoke<void>("portal_hide", { portal })
}

/** Brings a record, or what is new of one already followed. */
export function fetchRecord(portal: string, jurisdiction: string, number: string, year: string): Promise<PortalStatus> {
  return invoke<PortalStatus>("portal_fetch", { portal, jurisdiction, number, year })
}

export function updateAllRecords(portal: string): Promise<PortalStatus> {
  return invoke<PortalStatus>("portal_update_all", { portal })
}

export function stopPortal(): Promise<PortalStatus> {
  return invoke<PortalStatus>("portal_stop")
}

/** The person says they did what the portal asked. */
export function portalGoOn(): Promise<void> {
  return invoke<void>("portal_go_on")
}

export function setPortalManual(portal: string, enabled: boolean): Promise<PortalStatus> {
  return invoke<PortalStatus>("portal_manual", { portal, enabled })
}

export function getPortalStatus(): Promise<PortalStatus> {
  return invoke<PortalStatus>("portal_status")
}

export function listFollowed(portal: string): Promise<FollowedList> {
  return invoke<FollowedList>("portal_followed", { portal })
}

export function listJurisdictions(portal: string): Promise<Jurisdiction[]> {
  return invoke<Jurisdiction[]>("portal_jurisdictions", { portal })
}

/** Opens the folder of the portal, or of one of its records (its folder name). */
export function openPortalFolder(portal: string, record?: string): Promise<void> {
  return invoke<void>("portal_open_folder", { portal, record: record ?? null })
}

export function onPortalStatus(handler: (status: PortalStatus) => void): Promise<UnlistenFn> {
  return listen<PortalStatus>("portal://status", (event) => handler(event.payload))
}
