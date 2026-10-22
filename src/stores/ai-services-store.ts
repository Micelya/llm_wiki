/**
 * The registry of the AI services as the screens know it (P-141, D-443),
 * and the two ways it changes: when the app opens and when a person
 * chooses an engine in Ajustes → Servicios de IA.
 */
import { create } from "zustand"
import { getAiServices, setAiServices, type AiServices, type AiServicesView, type EnginesCatalog } from "@/commands/docbase"
import { NO_SERVICES, servicesFromSettings, settingsFromServices } from "@/lib/ai-services"
import { useWikiStore } from "@/stores/wiki-store"

interface AiServicesState {
  services: AiServices
  catalog: EnginesCatalog | null
  /** False until the registry is read from the base. */
  loaded: boolean
  setView: (view: AiServicesView) => void
}

export const useAiServicesStore = create<AiServicesState>((set) => ({
  services: NO_SERVICES,
  catalog: null,
  loaded: false,
  setView: (view) => set({ services: view.services, catalog: view.catalog, loaded: true }),
}))

/**
 * Writes the settings the chat and the research read from the registry,
 * only where they differ, and keeps them in the file.
 */
async function applyToSettings(services: AiServices): Promise<void> {
  const wiki = useWikiStore.getState()
  const next = settingsFromServices(services, wiki)
  const { saveActivePresetId, saveLlmConfig, saveProviderConfigs, saveSearchApiConfig } = await import("@/lib/project-store")
  if (JSON.stringify(next.providerConfigs) !== JSON.stringify(wiki.providerConfigs)) {
    wiki.setProviderConfigs(next.providerConfigs)
    await saveProviderConfigs(next.providerConfigs)
  }
  if (next.activePresetId !== wiki.activePresetId) {
    wiki.setActivePresetId(next.activePresetId)
    await saveActivePresetId(next.activePresetId)
  }
  if (JSON.stringify(next.llmConfig) !== JSON.stringify(wiki.llmConfig)) {
    wiki.setLlmConfig(next.llmConfig)
    await saveLlmConfig(next.llmConfig)
  }
  if (JSON.stringify(next.searchApiConfig) !== JSON.stringify(wiki.searchApiConfig)) {
    wiki.setSearchApiConfig(next.searchApiConfig)
    await saveSearchApiConfig(next.searchApiConfig)
  }
}

/**
 * When the app opens, after the settings are loaded: reads the registry.
 * The first time (it was never saved) it is made from the settings as
 * they are, so the same engines and models go on; the chat, when it was
 * set to the same connection, follows the service from then on.
 */
export async function initAiServices(): Promise<void> {
  const view = await getAiServices()
  if (view.saved) {
    useAiServicesStore.getState().setView(view)
    await applyToSettings(view.services)
    return
  }
  const wiki = useWikiStore.getState()
  const saved = await setAiServices(servicesFromSettings(wiki))
  useAiServicesStore.getState().setView(saved)
  const routing = wiki.taskModelRouting
  if (routing.chatPresetId && routing.chatPresetId === wiki.activePresetId) {
    const next = { ...routing, chatPresetId: null }
    wiki.setTaskModelRouting(next)
    const { saveTaskModelRouting } = await import("@/lib/project-store")
    await saveTaskModelRouting(next)
  }
}

let saving: Promise<void> = Promise.resolve()

/**
 * A person changed the registry: the base uses it from the next request,
 * and the chat and the research too. One change after the other, each on
 * the registry as the one before left it, so a quick second change never
 * undoes the first.
 */
export function updateAiServices(update: (current: AiServices) => AiServices): Promise<void> {
  const run = saving.then(async () => {
    const current = useAiServicesStore.getState().services
    const next = update(current)
    if (next === current) return
    const saved = await setAiServices(next)
    useAiServicesStore.getState().setView(saved)
    await applyToSettings(saved.services)
  })
  saving = run.catch(() => {})
  return run
}
