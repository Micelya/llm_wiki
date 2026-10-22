import { invoke } from "@tauri-apps/api/core"

/**
 * The connection with Micelya Core (D-463, D-468), kept in the system
 * keychain as one entry (`src-tauri/src/micelya_core/mod.rs`). The screens
 * never get its secret: only whether there is one.
 */
export interface CoreConnectionFields {
  /** The address of the API (`https://…/v1`). */
  api: string
  /** The issuer of the access permits (Keycloak: `https://…/realms/<name>`). */
  issuer: string
  /** The organization (Core's tenant). */
  tenant: string
  /** The technical user of the organization. */
  clientId: string
}

export interface CoreConnectionView extends CoreConnectionFields {
  /** The environment of Core it was given for; not edited here. */
  environment: string
  /** A secret is kept (it is never shown). */
  hasSecret: boolean
}

/** What «Probar la conexión» found (`ConnectionTest` in Rust). */
export interface CoreConnectionTest {
  ok: boolean
  /** The step that failed: the data on screen, the issuer, or the API. */
  step: "fields" | "permit" | "readings" | null
  problem: "fields" | "unreachable" | "timeout" | "rejected" | "organization" | "forbidden" | "not-found" | "unavailable" | "other" | null
  status: number | null
  /** What the other side said, never the secret nor the permit. */
  detail: string | null
  /** How many readings Core keeps for this credential. */
  readings: number | null
  moreReadings: boolean
}

/** The connection, without its secret; null when there is none. */
export function getCoreConnection(): Promise<CoreConnectionView | null> {
  return invoke<CoreConnectionView | null>("micelya_core_connection")
}

/** Keeps it in the keychain; without a new secret, the one kept stays. */
export function saveCoreConnection(fields: CoreConnectionFields, secret: string | null): Promise<CoreConnectionView> {
  return invoke<CoreConnectionView>("micelya_core_save_connection", { fields, secret })
}

export function removeCoreConnection(): Promise<void> {
  return invoke("micelya_core_remove_connection")
}

/** Asks for an access permit and the list of readings with these data, without keeping anything. */
export function testCoreConnection(fields: CoreConnectionFields, secret: string | null): Promise<CoreConnectionTest> {
  return invoke<CoreConnectionTest>("micelya_core_test_connection", { fields, secret })
}

/**
 * A reading this computer asked for in Micelya Core that Core refused to
 * delete (P-294; `LeftReading` in `micelya_core/ledger.rs`): Core keeps its
 * text until it is deleted. The app tries again by itself.
 */
export interface CoreLeftReading {
  id: string
  /** The generic name it was asked with (`pagina-N.jpg`). */
  name: string
  askedAt: number
  /** Its text was kept by the base: only deleting it is left. */
  kept: boolean
  /** Why Core refused to delete it, in its words. */
  refused: string | null
  triedAt: number | null
}

/** The readings of this computer that Micelya Core refused to delete. */
export function listCoreLeftReadings(): Promise<CoreLeftReading[]> {
  return invoke<CoreLeftReading[]>("micelya_core_left_readings")
}

/** Tries again to delete them now; how many Core still refuses to delete. */
export function deleteCoreLeftReadings(): Promise<number> {
  return invoke<number>("micelya_core_delete_left_readings")
}
