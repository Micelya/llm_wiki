/**
 * The connection with Micelya Core as the screens know it: its data
 * without the secret, read from the keychain when Ajustes opens and after
 * every change (`commands/micelya-core.ts`).
 */
import { create } from "zustand"
import {
  getCoreConnection,
  removeCoreConnection as removeKept,
  saveCoreConnection as saveKept,
  type CoreConnectionFields,
  type CoreConnectionView,
} from "@/commands/micelya-core"

interface CoreConnectionState {
  /** Null when there is none. */
  connection: CoreConnectionView | null
  /** False until it is read from the keychain. */
  loaded: boolean
  /** Why it could not be read, in words. */
  problem: string | null
}

export const useCoreConnectionStore = create<CoreConnectionState>(() => ({
  connection: null,
  loaded: false,
  problem: null,
}))

/** Reads it from the keychain. */
export async function loadCoreConnection(): Promise<void> {
  try {
    useCoreConnectionStore.setState({ connection: await getCoreConnection(), loaded: true, problem: null })
  } catch (cause) {
    useCoreConnectionStore.setState({ connection: null, loaded: true, problem: String(cause) })
  }
}

/** Keeps it; without a new secret, the one kept stays. */
export async function saveCoreConnection(fields: CoreConnectionFields, secret: string | null): Promise<CoreConnectionView> {
  const connection = await saveKept(fields, secret)
  useCoreConnectionStore.setState({ connection, loaded: true, problem: null })
  return connection
}

export async function removeCoreConnection(): Promise<void> {
  await removeKept()
  useCoreConnectionStore.setState({ connection: null, loaded: true, problem: null })
}
