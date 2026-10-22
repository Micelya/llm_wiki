/**
 * The registry of the AI services, seen from the screens (P-141, D-443).
 *
 * The registry lives in the base (`docbase/services.rs`): which engine
 * each service uses (asking the model, recognizing scans, researching on
 * the internet), with its model, its reasoning and an alternative. The
 * base reads it at every request. The chat and the research run here, with
 * the settings they always had (the active connection, the active web
 * search): those are copies the registry writes when it changes
 * (`settingsFromServices`), so everything that read them goes on working.
 *
 * Everything here is pure; saving is in `stores/ai-services-store.ts`.
 */
import type {
  AiServiceId,
  AiServices,
  AskConfig,
  CatalogEngine,
  EngineChoice,
  EnginesCatalog,
  RecognitionConfig,
} from "@/commands/docbase"
import { findLlmPreset } from "@/components/settings/llm-presets"
import { disabledLlmConfig, resolveConfig } from "@/components/settings/preset-resolver"
import { codexReasoningEffort } from "@/lib/codex-reasoning"
import { resolveSearchConfig } from "@/lib/web-search"
import type { CustomLlmPreset, LlmConfig, ProviderConfigs, SearchApiConfig, SearchProvider } from "@/stores/wiki-store"

export const NO_SERVICES: AiServices = {
  ask: { engine: null, alternative: null },
  recognize: { engine: null, alternative: null },
  research: { engine: null, alternative: null },
}

/** The engine of the catalog that a connection of a provider is. */
export function engineOfProvider(provider: LlmConfig["provider"]): string {
  switch (provider) {
    case "codex-cli":
    case "claude-code":
    case "openai":
    case "anthropic":
    case "google":
    case "azure":
    case "ollama":
      return provider
    default:
      return "custom"
  }
}

/** The settings the registry is made from, or writes to. */
export interface ServiceSettings {
  llmConfig: LlmConfig
  activePresetId: string | null
  providerConfigs: ProviderConfigs
  customLlmPresets: CustomLlmPreset[]
  searchApiConfig: SearchApiConfig
}

/**
 * The registry from the settings of before it, so opening the app changes
 * nothing: the base read and the chat answered with the model chosen in
 * Ajustes; the base asked and recognized scans with it only when it was the
 * Codex CLI; the research used the active web search.
 */
export function servicesFromSettings(settings: Pick<ServiceSettings, "llmConfig" | "activePresetId" | "searchApiConfig">): AiServices {
  const { llmConfig, activePresetId } = settings
  const codex = llmConfig.provider === "codex-cli"
  const connection = activePresetId ?? (codex ? "codex-cli" : null)
  const ask: EngineChoice | null = connection
    ? {
        engine: engineOfProvider(llmConfig.provider),
        connection,
        model: llmConfig.model.trim(),
        reasoning: codex ? codexReasoningEffort(llmConfig.codexReasoningEffort) : "",
      }
    : null
  const recognize: EngineChoice | null = codex
    ? { engine: "codex-cli", connection: connection ?? "codex-cli", model: llmConfig.model.trim(), reasoning: "" }
    : null
  const provider = settings.searchApiConfig.provider
  const research: EngineChoice | null =
    provider && provider !== "none" ? { engine: provider, connection: provider, model: "", reasoning: "" } : null
  return {
    ask: { engine: ask, alternative: null },
    recognize: { engine: recognize, alternative: null },
    research: { engine: research, alternative: null },
  }
}

/** What the base asks with (as `services::ask_engine` in Rust); null when it cannot. */
export function askConfigOf(services: AiServices): AskConfig | null {
  const choice = services.ask.engine
  if (choice?.engine !== "codex-cli") return null
  return choice.reasoning
    ? { engine: "codex-cli", model: choice.model, reasoning: codexReasoningEffort(choice.reasoning) }
    : { engine: "codex-cli", model: choice.model }
}

/** What the base recognizes scans with (as `services::recognition_engine`); null when it cannot. */
export function recognitionConfigOfServices(services: AiServices): RecognitionConfig | null {
  const choice = services.recognize.engine
  if (choice?.engine !== "codex-cli") return null
  return { engine: "codex-cli", model: choice.model }
}

/**
 * The settings the chat and the research read, written from the registry:
 * the active connection with the model and reasoning of «Preguntar al
 * modelo», and the active web search of «Investigar en internet». A
 * connection that no longer exists leaves the model settings as they are.
 */
export function settingsFromServices(services: AiServices, current: ServiceSettings): Omit<ServiceSettings, "customLlmPresets"> {
  let { llmConfig, activePresetId, providerConfigs } = current
  const ask = services.ask.engine
  if (!ask) {
    activePresetId = null
    llmConfig = disabledLlmConfig(llmConfig)
  } else {
    const preset = findLlmPreset(ask.connection, current.customLlmPresets)
    if (preset) {
      const saved = providerConfigs[ask.connection] ?? {}
      const override = { ...saved }
      // The same model written with spaces around (while it is typed in
      // Conexiones) is left as it is.
      if (ask.model && ask.model !== (saved.model ?? "").trim()) override.model = ask.model
      if (preset.provider === "codex-cli" && ask.reasoning && ask.reasoning !== saved.codexReasoningEffort) {
        override.codexReasoningEffort = codexReasoningEffort(ask.reasoning)
      }
      providerConfigs = { ...providerConfigs, [ask.connection]: override }
      activePresetId = ask.connection
      llmConfig = resolveConfig(preset, override, llmConfig)
    }
  }
  const provider = (services.research.engine?.engine ?? "none") as SearchProvider
  const resolved = resolveSearchConfig(current.searchApiConfig)
  const searchApiConfig = resolved.provider === provider ? current.searchApiConfig : resolveSearchConfig({ ...resolved, provider })
  return { llmConfig, activePresetId, providerConfigs, searchApiConfig }
}

/**
 * The registry after a connection of Conexiones changed: when it is the
 * one «Preguntar al modelo» uses, its model and reasoning go with it.
 */
export function servicesAfterConnectionChange(
  services: AiServices,
  connection: string,
  change: { model?: string; codexReasoningEffort?: string },
): AiServices {
  const ask = services.ask.engine
  if (!ask || ask.connection !== connection) return services
  const next: EngineChoice = { ...ask }
  if (change.model !== undefined) next.model = change.model.trim()
  if (change.codexReasoningEffort !== undefined && ask.engine === "codex-cli") next.reasoning = codexReasoningEffort(change.codexReasoningEffort)
  if (next.model === ask.model && next.reasoning === ask.reasoning) return services
  return { ...services, ask: { ...services.ask, engine: next } }
}

/** The services that use a connection, by their id. */
export function servicesUsing(services: AiServices, connection: string): AiServiceId[] {
  return (["ask", "recognize", "research"] as const).filter((id) => {
    const { engine, alternative } = services[id]
    return engine?.connection === connection || alternative?.connection === connection
  })
}

/** The entry of the catalog of an engine for a service. */
export function catalogEngine(catalog: EnginesCatalog | null, service: AiServiceId, engine: string | undefined): CatalogEngine | null {
  if (!catalog || !engine) return null
  return catalog.engines.find((entry) => entry.service === service && entry.id === engine) ?? null
}

/** The name of a service in words. */
export function serviceName(catalog: EnginesCatalog | null, service: AiServiceId): string {
  return catalog?.services.find((entry) => entry.id === service)?.name ?? service
}

/** «Codex (gpt-6-luna, razonamiento alto)», «Tavily»: a choice in words. */
export function choiceName(catalog: EnginesCatalog | null, service: AiServiceId, choice: EngineChoice | null, connectionLabel?: string): string {
  if (!choice) return "Ninguno"
  const name = connectionLabel ?? catalogEngine(catalog, service, choice.engine)?.name ?? choice.engine
  const details = [choice.model, choice.reasoning ? `razonamiento ${REASONING_NAMES[choice.reasoning] ?? choice.reasoning}` : ""].filter(Boolean)
  return details.length ? `${name} (${details.join(", ")})` : name
}

export const REASONING_NAMES: Record<string, string> = { low: "bajo", medium: "medio", high: "alto" }

export const CAPABILITY_LEVEL_NAMES: Record<string, string> = { yes: "sí", partial: "a veces", no: "no" }
