import type { PortalProgress, PortalStatus } from "@/commands/portals"

/** Invented states of the walk over a record, for the tests. */

export function progressOf(patch: Partial<PortalProgress> = {}): PortalProgress {
  return {
    state: "working",
    message: "Bajando ZZZ4321-2011_fs6-7_2026-11-08_FIRMA-DESPACHO_Proveido-inventado.pdf",
    reference: "ZZZ 4321/2011",
    person: null,
    busy: false,
    page: 2,
    knownFrom: null,
    seen: 20,
    withDocument: 12,
    downloaded: 9,
    already: 1,
    failed: 0,
    pending: 2,
    current: "ZZZ4321-2011_fs6-7_2026-11-08_FIRMA-DESPACHO_Proveido-inventado.pdf",
    warnings: [],
    ...patch,
  }
}

export function statusOf(patch: Partial<PortalStatus> = {}, progress: Partial<PortalProgress> = {}): PortalStatus {
  return { running: "ZZZ 4321/2011", queue: [], manual: false, progress: progressOf(progress), ...patch }
}
