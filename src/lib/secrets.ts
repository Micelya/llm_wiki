/**
 * The API keys live in the system keychain (Credential Manager on Windows),
 * not in `app-state.json` (D-444, `src-tauri/src/secrets.rs`).
 *
 * The app keeps the keys in memory, as before: they are read from the
 * keychain once, when it opens, and written there on every change. The
 * settings file keeps everything else, with the key empty. When the
 * keychain cannot be used, a key stays in the file as it always did: it is
 * never lost because the keychain failed.
 */
import { invoke } from "@tauri-apps/api/core"

export type KeyMap = Record<string, string>

/** The key of the main model (`llmConfig.apiKey`). */
export const LLM_KEY = "llm"
/** The key of the web search (`searchApiConfig.apiKey`). */
export const SEARCH_KEY = "search"
/** The key of a connection with a provider (`providerConfigs.<id>.apiKey`). */
export const providerKey = (id: string): string => `provider:${id}`
/** The key of a search provider (`searchApiConfig.providerConfigs.<id>.apiKey`). */
export const searchProviderKey = (id: string): string => `search:${id}`

/** The keys the keychain had when the app opened; null when it could not be read. */
let known: KeyMap | null = null

/** The names of every key the settings may have, by where each one is (as `places` in Rust). */
export function keyNamesOf(settings: {
  llmConfig?: unknown
  providerConfigs?: unknown
  searchApiConfig?: unknown
}): string[] {
  const names: string[] = []
  const isObject = (value: unknown): value is Record<string, unknown> =>
    !!value && typeof value === "object" && !Array.isArray(value)
  if (isObject(settings.llmConfig)) names.push(LLM_KEY)
  if (isObject(settings.providerConfigs)) {
    for (const [id, config] of Object.entries(settings.providerConfigs)) if (isObject(config)) names.push(providerKey(id))
  }
  if (isObject(settings.searchApiConfig)) {
    names.push(SEARCH_KEY)
    const configs = settings.searchApiConfig.providerConfigs
    if (isObject(configs)) {
      for (const [id, config] of Object.entries(configs)) if (isObject(config)) names.push(searchProviderKey(id))
    }
  }
  return names
}

/** Reads these keys from the keychain; false (and the keys stay where they are) when it cannot. */
export async function loadKeys(names: string[]): Promise<boolean> {
  try {
    known = await invoke<KeyMap>("secrets_read", { names })
    return true
  } catch (error) {
    console.warn("[secrets] the keychain could not be read; the keys stay in the settings file:", error)
    known = null
    return false
  }
}

/** The key the keychain had with this name. */
export function knownKey(name: string): string | undefined {
  return known?.[name] || undefined
}

/**
 * Keeps these keys in the keychain (an empty one is removed) and returns
 * the names whose key can leave the file: the keychain kept it, or already
 * had it. Without a keychain, none: every key stays in the file.
 */
export async function keepKeys(entries: KeyMap): Promise<Set<string>> {
  if (known === null) return new Set()
  const current = known
  const free = new Set<string>()
  const changed: KeyMap = {}
  for (const [name, value] of Object.entries(entries)) {
    if ((current[name] ?? "") === value) free.add(name)
    else changed[name] = value
  }
  if (Object.keys(changed).length === 0) return free
  let kept: string[] = []
  try {
    kept = await invoke<string[]>("secrets_write", { entries: changed })
  } catch (error) {
    console.warn("[secrets] the keychain could not keep the keys; they stay in the settings file:", error)
  }
  for (const name of kept) {
    if (!(name in changed)) continue
    if (changed[name]) current[name] = changed[name]
    else delete current[name]
    free.add(name)
  }
  return free
}

/** The object with its key emptied, to be written in the file (unchanged when it has none). */
export function withoutKey<T extends { apiKey?: string }>(value: T): T {
  return value.apiKey ? { ...value, apiKey: "" } : value
}

/** The object with the key the keychain has, when the file has none. */
export function withKey<T extends { apiKey?: string }>(value: T, name: string): T {
  if (value.apiKey) return value
  const key = knownKey(name)
  return key ? { ...value, apiKey: key } : value
}

export const __secretsTest = {
  reset(keys: KeyMap | null) {
    known = keys === null ? null : { ...keys }
  },
  known: () => known,
}
