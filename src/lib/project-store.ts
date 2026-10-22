/**
 * The app's settings, kept in `app-state.json` in the app's data folder.
 * They are all global: there are no projects (W6). The API keys are not
 * written here but in the system keychain (`secrets.ts`, D-444): the
 * file keeps them empty, unless the keychain cannot be used.
 */
import { load } from "@tauri-apps/plugin-store"
import type { CustomLlmPreset, GeneralConfig, LlmConfig, SearchApiConfig, OutputLanguage, ProviderConfigs, ProxyConfig, TaskModelRoutingConfig } from "@/stores/wiki-store"
import { DEFAULT_ZOOM_LEVEL, clampZoomLevel } from "@/stores/zoom-store"
import {
  LLM_KEY,
  SEARCH_KEY,
  keepKeys,
  keyNamesOf,
  loadKeys,
  providerKey,
  searchProviderKey,
  withKey,
  withoutKey,
} from "@/lib/secrets"

const STORE_NAME = "app-state.json"

async function getStore() {
  return load(STORE_NAME, { autoSave: true, defaults: {} })
}

const LLM_CONFIG_KEY = "llmConfig"
const PROVIDER_CONFIGS_KEY = "providerConfigs"
const ACTIVE_PRESET_KEY = "activePresetId"
const TASK_MODEL_ROUTING_KEY = "taskModelRouting"
const CUSTOM_LLM_PRESETS_KEY = "customLlmPresets"
let customLlmPresetWrite = Promise.resolve()

/**
 * Reads from the keychain the keys of the settings saved, once, when the
 * app opens and before loading them. False when the keychain cannot be
 * read: then the keys stay in the file.
 */
export async function loadSettingsKeys(): Promise<boolean> {
  const store = await getStore()
  return loadKeys(
    keyNamesOf({
      llmConfig: await store.get(LLM_CONFIG_KEY),
      providerConfigs: await store.get(PROVIDER_CONFIGS_KEY),
      searchApiConfig: await store.get(SEARCH_API_KEY),
    }),
  )
}

export async function saveLlmConfig(config: LlmConfig): Promise<void> {
  const store = await getStore()
  const free = await keepKeys({ [LLM_KEY]: config.apiKey ?? "" })
  await store.set(LLM_CONFIG_KEY, free.has(LLM_KEY) ? withoutKey(config) : config)
}

export async function loadLlmConfig(): Promise<LlmConfig | null> {
  const store = await getStore()
  const config = await store.get<LlmConfig>(LLM_CONFIG_KEY)
  return config ? withKey(config, LLM_KEY) : null
}

/** The connections without the keys the keychain keeps. */
export async function providerConfigsForFile(configs: ProviderConfigs): Promise<ProviderConfigs> {
  const free = await keepKeys(
    Object.fromEntries(Object.entries(configs).map(([id, config]) => [providerKey(id), config.apiKey ?? ""])),
  )
  return Object.fromEntries(
    Object.entries(configs).map(([id, config]) => [id, free.has(providerKey(id)) ? withoutKey(config) : config]),
  )
}

export async function saveProviderConfigs(configs: ProviderConfigs): Promise<void> {
  const store = await getStore()
  await store.set(PROVIDER_CONFIGS_KEY, await providerConfigsForFile(configs))
}

export async function loadProviderConfigs(): Promise<ProviderConfigs | null> {
  const store = await getStore()
  const configs = await store.get<ProviderConfigs>(PROVIDER_CONFIGS_KEY)
  if (!configs) return null
  return Object.fromEntries(Object.entries(configs).map(([id, config]) => [id, withKey(config, providerKey(id))]))
}

export async function saveCustomLlmPresets(presets: CustomLlmPreset[]): Promise<void> {
  const normalized = normalizeCustomLlmPresets(presets)
  const write = customLlmPresetWrite.then(async () => {
    const store = await getStore()
    await store.set(CUSTOM_LLM_PRESETS_KEY, normalized)
  })
  customLlmPresetWrite = write.catch(() => {})
  await write
}

export async function loadCustomLlmPresets(): Promise<CustomLlmPreset[]> {
  const store = await getStore()
  return normalizeCustomLlmPresets(await store.get<unknown>(CUSTOM_LLM_PRESETS_KEY))
}

function normalizeCustomLlmPresets(value: unknown): CustomLlmPreset[] {
  if (!Array.isArray(value)) return []
  const seen = new Set<string>()
  const normalized: CustomLlmPreset[] = []
  for (const entry of value) {
    if (normalized.length >= 50) break
    const candidate = entry as Partial<CustomLlmPreset> | null
    if (!candidate || typeof candidate !== "object") continue
    if (typeof candidate.id !== "string" || !/^custom-[A-Za-z0-9-]{1,80}$/.test(candidate.id)) continue
    const label = typeof candidate.label === "string" ? candidate.label.trim().slice(0, 80) : ""
    if (!label || seen.has(candidate.id)) continue
    seen.add(candidate.id)
    normalized.push({ id: candidate.id, label })
  }
  return normalized
}

export async function saveActivePresetId(id: string | null): Promise<void> {
  const store = await getStore()
  await store.set(ACTIVE_PRESET_KEY, id)
}

export async function loadActivePresetId(): Promise<string | null> {
  const store = await getStore()
  return (await store.get<string | null>(ACTIVE_PRESET_KEY)) ?? null
}

export async function saveTaskModelRouting(config: TaskModelRoutingConfig): Promise<void> {
  const store = await getStore()
  await store.set(TASK_MODEL_ROUTING_KEY, config)
}

export async function loadTaskModelRouting(): Promise<TaskModelRoutingConfig | null> {
  const store = await getStore()
  const saved = await store.get<Partial<TaskModelRoutingConfig>>(TASK_MODEL_ROUTING_KEY)
  if (!saved) return null
  return {
    chatPresetId: typeof saved.chatPresetId === "string" ? saved.chatPresetId : null,
    ingestPresetId: typeof saved.ingestPresetId === "string" ? saved.ingestPresetId : null,
  }
}

const SEARCH_API_KEY = "searchApiConfig"

/** The web search without the keys the keychain keeps. */
export async function searchApiConfigForFile(config: SearchApiConfig): Promise<SearchApiConfig> {
  const providers = Object.entries(config.providerConfigs ?? {})
  const free = await keepKeys({
    [SEARCH_KEY]: config.apiKey ?? "",
    ...Object.fromEntries(providers.map(([id, override]) => [searchProviderKey(id), override?.apiKey ?? ""])),
  })
  const forFile = free.has(SEARCH_KEY) ? withoutKey(config) : config
  if (!config.providerConfigs) return forFile
  return {
    ...forFile,
    providerConfigs: Object.fromEntries(
      providers.map(([id, override]) => [id, override && free.has(searchProviderKey(id)) ? withoutKey(override) : override]),
    ),
  }
}

export async function saveSearchApiConfig(config: SearchApiConfig): Promise<void> {
  const store = await getStore()
  await store.set(SEARCH_API_KEY, await searchApiConfigForFile(config))
}

export async function loadSearchApiConfig(): Promise<SearchApiConfig | null> {
  const store = await getStore()
  const config = await store.get<SearchApiConfig>(SEARCH_API_KEY)
  if (!config) return null
  const filled = withKey(config, SEARCH_KEY)
  if (!filled.providerConfigs) return filled
  return {
    ...filled,
    providerConfigs: Object.fromEntries(
      Object.entries(filled.providerConfigs).map(([id, override]) => [id, override ? withKey(override, searchProviderKey(id)) : override]),
    ),
  }
}

function normalizeZoomLevel(level: unknown): number {
  return typeof level === "number" && Number.isFinite(level)
    ? clampZoomLevel(level)
    : DEFAULT_ZOOM_LEVEL
}

export const __projectStoreTest = {
  normalizeZoomLevel,
  normalizeCustomLlmPresets,
}

// IMPORTANT: Keep this key in sync with the Rust setup hook
// (src-tauri/src/proxy.rs), which reads this exact field name from
// the same `app-state.json` store at app launch to translate the
// config into HTTP_PROXY / HTTPS_PROXY / NO_PROXY env vars.
const PROXY_CONFIG_KEY = "proxyConfig"

export async function saveProxyConfig(config: ProxyConfig): Promise<void> {
  const store = await getStore()
  await store.set(PROXY_CONFIG_KEY, config)
  // Force-flush to disk. The store is opened with `autoSave: true`,
  // which is a 100ms debounce — not an immediate write. For most
  // settings that's fine, but the proxy config is on the startup
  // critical path: the Rust setup hook reads `app-state.json` on
  // launch to apply HTTP_PROXY / HTTPS_PROXY / NO_PROXY. If the
  // user saves and quits within the debounce window the disk
  // value would lag behind in-memory, and the next launch would
  // boot with the wrong proxy.
  await store.save()
}

export async function loadProxyConfig(): Promise<ProxyConfig | null> {
  const store = await getStore()
  return (await store.get<ProxyConfig>(PROXY_CONFIG_KEY)) ?? null
}

const GENERAL_CONFIG_KEY = "generalConfig"

export const DEFAULT_GENERAL_CONFIG: GeneralConfig = {
  autostart: false,
  closeBehavior: "minimize",
}

export function normalizeGeneralConfig(config?: Partial<GeneralConfig> | null): GeneralConfig {
  const closeBehavior = config?.closeBehavior
  return {
    autostart: typeof config?.autostart === "boolean" ? config.autostart : DEFAULT_GENERAL_CONFIG.autostart,
    closeBehavior:
      closeBehavior === "ask" || closeBehavior === "minimize" || closeBehavior === "exit"
        ? closeBehavior
        : DEFAULT_GENERAL_CONFIG.closeBehavior,
  }
}

export async function saveGeneralConfig(config: GeneralConfig): Promise<void> {
  const store = await getStore()
  await store.set(GENERAL_CONFIG_KEY, normalizeGeneralConfig(config))
  await store.save()
}

export async function loadGeneralConfig(): Promise<GeneralConfig> {
  const store = await getStore()
  const config = await store.get<Partial<GeneralConfig>>(GENERAL_CONFIG_KEY)
  return normalizeGeneralConfig(config)
}

const LANGUAGE_KEY = "language"

export async function saveLanguage(lang: string): Promise<void> {
  const store = await getStore()
  await store.set(LANGUAGE_KEY, lang)
}

export async function loadLanguage(): Promise<string | null> {
  const store = await getStore()
  return (await store.get<string>(LANGUAGE_KEY)) ?? null
}

const THEME_KEY = "theme"

export async function saveTheme(theme: "light" | "dark" | "system"): Promise<void> {
  const store = await getStore()
  await store.set(THEME_KEY, theme)
}

export async function loadTheme(): Promise<"light" | "dark" | "system" | null> {
  const store = await getStore()
  return (await store.get<"light" | "dark" | "system">(THEME_KEY)) ?? null
}

const OUTPUT_LANGUAGE_KEY = "outputLanguage"

export async function saveOutputLanguage(lang: OutputLanguage): Promise<void> {
  const store = await getStore()
  await store.set(OUTPUT_LANGUAGE_KEY, lang)
}

export async function loadOutputLanguage(): Promise<OutputLanguage | null> {
  const store = await getStore()
  return (await store.get<OutputLanguage>(OUTPUT_LANGUAGE_KEY)) ?? null
}

// ── Update-check persistence ──────────────────────────────────────────────
// Small slice of state the UI-layer update store hydrates from on boot.
// Only fields that should persist across launches: the user's "enable
// auto-check" toggle, the timestamp we last checked (so the 6-hour cache
// survives restarts), and the version the user explicitly dismissed
// (so we don't re-nag on every restart until a newer version is out).

const UPDATE_CHECK_STATE_KEY = "updateCheckState"

export interface PersistedUpdateCheckState {
  enabled: boolean
  lastCheckedAt: number | null
  dismissedVersion: string | null
}

export async function saveUpdateCheckState(
  state: PersistedUpdateCheckState,
): Promise<void> {
  const store = await getStore()
  await store.set(UPDATE_CHECK_STATE_KEY, state)
}

export async function loadUpdateCheckState(): Promise<PersistedUpdateCheckState | null> {
  const store = await getStore()
  return (
    (await store.get<PersistedUpdateCheckState>(UPDATE_CHECK_STATE_KEY)) ?? null
  )
}

const ZOOM_LEVEL_KEY = "zoomLevel"

export async function saveZoomLevel(level: number): Promise<void> {
  const store = await getStore()
  await store.set(ZOOM_LEVEL_KEY, normalizeZoomLevel(level))
  await store.save()
}

export async function loadZoomLevel(): Promise<number> {
  const store = await getStore()
  const level = await store.get<number>(ZOOM_LEVEL_KEY)
  return normalizeZoomLevel(level)
}
