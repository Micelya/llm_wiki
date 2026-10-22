import { describe, expect, it } from "vitest"
import type { AiServices, EngineChoice } from "@/commands/docbase"
import { findLlmPreset } from "@/components/settings/llm-presets"
import { resolveConfig } from "@/components/settings/preset-resolver"
import type { LlmConfig, ProviderConfigs, SearchApiConfig } from "@/stores/wiki-store"
import {
  NO_SERVICES,
  askConfigOf,
  recognitionConfigOfServices,
  servicesAfterConnectionChange,
  servicesFromSettings,
  servicesUsing,
  settingsFromServices,
  type ServiceSettings,
} from "./ai-services"

// Every model name and key below is made up.
const base: LlmConfig = {
  provider: "openai",
  apiKey: "",
  model: "",
  ollamaUrl: "http://localhost:11434",
  customEndpoint: "",
  maxContextSize: 8192,
}
const noSearch: SearchApiConfig = { provider: "none", apiKey: "" }

/** The settings as the app leaves them when a connection is active (App.tsx resolves it on start). */
function settingsWith(activePresetId: string | null, providerConfigs: ProviderConfigs, searchApiConfig = noSearch): ServiceSettings {
  const preset = activePresetId ? findLlmPreset(activePresetId) : undefined
  const llmConfig = preset ? resolveConfig(preset, providerConfigs[activePresetId!], base) : base
  return { llmConfig, activePresetId, providerConfigs, customLlmPresets: [], searchApiConfig }
}

/** What the app asked and recognized with before the registry (document-asking.ts and text-recognition.ts). */
function before(llmConfig: LlmConfig) {
  if (llmConfig.provider !== "codex-cli") return { ask: null, recognize: null }
  return {
    ask: { engine: "codex-cli", model: llmConfig.model, reasoning: llmConfig.codexReasoningEffort ?? "high" },
    recognize: { engine: "codex-cli", model: llmConfig.model },
  }
}

function choice(engine: string, connection: string, model = "", reasoning = ""): EngineChoice {
  return { engine, connection, model, reasoning }
}

describe("the registry made from the settings of before it", () => {
  it("keeps the same engines and models for the base", () => {
    const cases: ServiceSettings[] = [
      settingsWith("codex-cli", { "codex-cli": { model: "modelo-a", codexReasoningEffort: "medium" } }),
      settingsWith("codex-cli", {}),
      settingsWith("openai", { openai: { apiKey: "clave-x", model: "modelo-b" } }),
      settingsWith("claude-code-cli", {}),
      settingsWith(null, {}),
    ]
    for (const settings of cases) {
      const services = servicesFromSettings(settings)
      const old = before(settings.llmConfig)
      expect(askConfigOf(services)).toEqual(old.ask)
      expect(recognitionConfigOfServices(services)).toEqual(old.recognize)
    }
  })

  it("names the engine and the connection each service uses", () => {
    const codex = servicesFromSettings(
      settingsWith("codex-cli", { "codex-cli": { model: "modelo-a", codexReasoningEffort: "high" } }, {
        provider: "tavily",
        apiKey: "",
      }),
    )
    expect(codex.ask.engine).toEqual(choice("codex-cli", "codex-cli", "modelo-a", "high"))
    expect(codex.recognize.engine).toEqual(choice("codex-cli", "codex-cli", "modelo-a"))
    expect(codex.research.engine).toEqual(choice("tavily", "tavily"))
    expect(codex.ask.alternative).toBeNull()

    const openai = servicesFromSettings(settingsWith("openai", { openai: { model: "modelo-b" } }))
    expect(openai.ask.engine).toEqual(choice("openai", "openai", "modelo-b"))
    expect(openai.recognize.engine).toBeNull()
    expect(openai.research.engine).toBeNull()

    const custom = servicesFromSettings({ ...settingsWith(null, {}), activePresetId: "custom-propio", llmConfig: { ...base, provider: "custom", model: "m" } })
    expect(custom.ask.engine).toEqual(choice("custom", "custom-propio", "m"))

    expect(servicesFromSettings(settingsWith(null, {}))).toEqual(NO_SERVICES)
  })

  it("written back, leaves the chat and the research as they were", () => {
    const settings = settingsWith(
      "codex-cli",
      { "codex-cli": { model: "modelo-a", codexReasoningEffort: "low" }, openai: { apiKey: "clave-x" } },
      { provider: "tavily", apiKey: "", providerConfigs: { tavily: { apiKey: "clave-y" } } },
    )
    const next = settingsFromServices(servicesFromSettings(settings), settings)
    expect(next.llmConfig).toEqual(settings.llmConfig)
    expect(next.activePresetId).toBe("codex-cli")
    expect(next.providerConfigs).toEqual(settings.providerConfigs)
    expect(next.searchApiConfig).toBe(settings.searchApiConfig)
  })
})

describe("changing the engine of a service", () => {
  const settings = settingsWith("codex-cli", { "codex-cli": { model: "modelo-a" }, "claude-code-cli": { model: "modelo-c" } })

  it("reaches the chat with its model, and the base only when it can run it", () => {
    const services: AiServices = { ...NO_SERVICES, ask: { engine: choice("claude-code", "claude-code-cli", "modelo-d"), alternative: null } }
    const next = settingsFromServices(services, settings)
    expect(next.activePresetId).toBe("claude-code-cli")
    expect(next.llmConfig.provider).toBe("claude-code")
    expect(next.llmConfig.model).toBe("modelo-d")
    expect(next.providerConfigs["claude-code-cli"].model).toBe("modelo-d")
    expect(askConfigOf(services)).toBeNull()

    const codex: AiServices = { ...NO_SERVICES, ask: { engine: choice("codex-cli", "codex-cli", "modelo-e", "medium"), alternative: null } }
    const back = settingsFromServices(codex, { ...next, customLlmPresets: [] })
    expect(back.llmConfig.provider).toBe("codex-cli")
    expect(back.llmConfig.model).toBe("modelo-e")
    expect(back.llmConfig.codexReasoningEffort).toBe("medium")
    expect(askConfigOf(codex)).toEqual({ engine: "codex-cli", model: "modelo-e", reasoning: "medium" })
  })

  it("to none leaves the chat without a model and the research without a search", () => {
    const withSearch = { ...settings, searchApiConfig: { provider: "tavily", apiKey: "", providerConfigs: { tavily: { apiKey: "clave-y" } } } as SearchApiConfig }
    const next = settingsFromServices(NO_SERVICES, withSearch)
    expect(next.activePresetId).toBeNull()
    expect(next.llmConfig.provider).toBe("openai")
    expect(next.llmConfig.apiKey).toBe("")
    expect(next.searchApiConfig.provider).toBe("none")
    expect(next.searchApiConfig.providerConfigs?.tavily?.apiKey).toBe("clave-y")
    const research: AiServices = { ...NO_SERVICES, research: { engine: choice("tavily", "tavily"), alternative: null } }
    expect(settingsFromServices(research, { ...withSearch, searchApiConfig: next.searchApiConfig }).searchApiConfig.provider).toBe("tavily")
  })

  it("follows the model and reasoning edited in Conexiones for the connection in use", () => {
    const services: AiServices = { ...NO_SERVICES, ask: { engine: choice("codex-cli", "codex-cli", "modelo-a", "high"), alternative: null } }
    const edited = servicesAfterConnectionChange(services, "codex-cli", { model: " modelo-f ", codexReasoningEffort: "low" })
    expect(edited.ask.engine).toEqual(choice("codex-cli", "codex-cli", "modelo-f", "low"))
    expect(servicesAfterConnectionChange(services, "openai", { model: "otro" })).toBe(services)
    expect(servicesAfterConnectionChange(services, "codex-cli", { model: "modelo-a" })).toBe(services)
  })

  it("tells which services use a connection", () => {
    const services: AiServices = {
      ask: { engine: choice("codex-cli", "codex-cli", "m"), alternative: choice("openai", "openai", "n") },
      recognize: { engine: choice("codex-cli", "codex-cli", "m"), alternative: null },
      research: { engine: choice("tavily", "tavily"), alternative: null },
    }
    expect(servicesUsing(services, "codex-cli")).toEqual(["ask", "recognize"])
    expect(servicesUsing(services, "openai")).toEqual(["ask"])
    expect(servicesUsing(services, "tavily")).toEqual(["research"])
    expect(servicesUsing(services, "google")).toEqual([])
  })
})
