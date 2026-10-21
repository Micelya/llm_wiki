import type { CustomApiMode } from "./llm-presets"
import type { AzureModelFamily, CloseBehavior, ReasoningConfig } from "@/stores/wiki-store"

/**
 * Shape of the draft state each section reads from and writes into.
 * The parent (SettingsView) owns one instance and hands it to every
 * section; the Save button at the bottom flushes the whole draft to
 * stores + disk in one commit.
 */
export interface SettingsDraft {
  // LLM provider
  provider: "openai" | "anthropic" | "google" | "azure" | "ollama" | "custom" | "minimax" | "claude-code" | "codex-cli"
  apiKey: string
  model: string
  ollamaUrl: string
  customEndpoint: string
  azureApiVersion: string
  azureModelFamily: AzureModelFamily
  maxContextSize: number
  apiMode: CustomApiMode | undefined
  reasoning: ReasoningConfig | undefined
  ingestReasoning: ReasoningConfig | undefined
  localCliIsolation: boolean

  // Output preferences
  outputLanguage: string
  maxHistoryMessages: number

  // Network — global outbound HTTP proxy. Persisted to app-state.json
  // and read by the Rust setup hook on app launch (changes apply
  // after restart). See src/lib/proxy-config.ts.
  proxyEnabled: boolean
  proxyUrl: string
  proxyBypassLocal: boolean
  proxyAcceptInvalidCerts: boolean

  // UI
  uiLanguage: string
  theme: "light" | "dark" | "system"
  zoomLevel: number

  // General app behavior
  autostart: boolean
  closeBehavior: CloseBehavior
}

export type DraftSetter = <K extends keyof SettingsDraft>(
  key: K,
  value: SettingsDraft[K],
) => void
