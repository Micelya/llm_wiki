import type { CodexReasoningEffort } from "@/lib/codex-reasoning"
import { create } from "zustand"
import { DEFAULT_GRAPH_FILTERS, type GraphFilterState } from "@/lib/graph-filters"
import type { OutputLanguage } from "@/lib/output-language-options"

/**
 * Wire protocol used when `provider === "custom"`. Other providers have a
 * fixed protocol (openai → OpenAI chat; anthropic → Anthropic messages;
 * etc.), so this field is ignored for them. `undefined` defaults to
 * `chat_completions` for backward compatibility with pre-0.3.7 configs.
 */
export type CustomApiMode = "chat_completions" | "anthropic_messages"
export type AzureModelFamily = "auto" | "gpt5"
export type ReasoningMode = "auto" | "off" | "low" | "medium" | "high" | "max" | "custom"

export interface ReasoningConfig {
  mode: ReasoningMode
  budgetTokens?: number
}

interface LlmConfig {
  provider: "openai" | "anthropic" | "google" | "azure" | "ollama" | "custom" | "minimax" | "claude-code" | "codex-cli"
  apiKey: string
  model: string
  ollamaUrl: string
  customEndpoint: string
  azureApiVersion?: string
  azureModelFamily?: AzureModelFamily
  maxContextSize: number // max context window in characters
  apiMode?: CustomApiMode
  reasoning?: ReasoningConfig
  /**
   * Reasoning for ingest's structured calls (analysis, page generation,
   * captioning, dedup), kept separate from `reasoning` because the two want
   * opposite things: chat benefits from thinking, whereas ingest asks for
   * structured output thousands of times and a model that spends its budget on
   * chain-of-thought can end the stream with empty `content` — a lost page.
   * Defaults to "off", which is what ingest hardcoded before this was settable.
   */
  ingestReasoning?: ReasoningConfig
  /**
   * Local CLI providers only. When true, the app asks Claude/Codex CLI
   * to ignore user-level rules/config/MCP/tool state where the CLI exposes
   * such controls. Default false preserves existing advanced-user setups.
   */
  localCliIsolation?: boolean
  /** Codex CLI provider only. Overall subprocess timeout in minutes. */
  codexCliTimeoutMinutes?: number
  /** Codex CLI provider only. How much the model reasons (D-274); missing means the default. */
  codexReasoningEffort?: CodexReasoningEffort
  /** HTTP LLM request backstop. Defaults to 30 minutes for legacy configs. */
  requestTimeoutMinutes?: number
  /** Defaults to true. HTTP providers use a non-streaming wire when false. */
  streamingEnabled?: boolean
  /** Optional headers added to every HTTP request for this provider preset. */
  customHeaders?: Record<string, string>
}

export type SearchProvider =
  | "tavily"
  | "serpapi"
  | "searxng"
  | "ollama"
  | "brave"
  | "bocha"
  | "firecrawl"
  | "none"
export type DeepResearchSource = "web" | "anytxt" | "both"
export type SerpApiEngine =
  | "google"
  | "google_news"
  | "google_scholar"
  | "google_patents"
  | "bing"
  | "duckduckgo"
  | "google_images"
  | "google_videos"
  | "youtube"
  | string
export type SearXngCategory =
  | "general"
  | "news"
  | "science"
  | "it"
  | "images"
  | "videos"
  | "files"
  | "map"
  | "music"
  | "social media"
  | string

export interface SearchProviderOverride {
  apiKey?: string
  baseUrl?: string
  serpApiEngine?: SerpApiEngine
  searXngUrl?: string
  searXngCategories?: SearXngCategory[]
  ollamaUrl?: string
}

export type SearchProviderConfigs = Partial<Record<Exclude<SearchProvider, "none">, SearchProviderOverride>>

export interface AnyTxtConfig {
  enabled?: boolean
  endpoint?: string
  filterDir?: string
  filterExt?: string
  limit?: number
}

interface SearchApiConfig {
  provider: SearchProvider
  apiKey: string
  serpApiEngine?: SerpApiEngine
  searXngUrl?: string
  searXngCategories?: SearXngCategory[]
  ollamaUrl?: string
  providerConfigs?: SearchProviderConfigs
  deepResearchSource?: DeepResearchSource
  anyTxt?: AnyTxtConfig
}

/**
 * Global outbound HTTP proxy. When `enabled` and `url` is a valid
 * http(s) URL, the Rust setup hook reads this on app launch and
 * sets HTTP_PROXY / HTTPS_PROXY / NO_PROXY env vars before the
 * reqwest client used by tauri-plugin-http is constructed. Changes
 * apply on app restart only.
 */
interface ProxyConfig {
  enabled: boolean
  url: string
  bypassLocal: boolean
  acceptInvalidCerts?: boolean
}

export type CloseBehavior = "ask" | "minimize" | "exit"

export type GraphColorMode = "type" | "community"

export interface GraphUiState {
  colorMode: GraphColorMode
  filters: GraphFilterState
  nodeScale: number
  graphSpacingDraft: number
}

export function createDefaultGraphUiState(): GraphUiState {
  return {
    colorMode: "type",
    filters: {
      ...DEFAULT_GRAPH_FILTERS,
      hiddenTypes: new Set(),
      hiddenNodeIds: new Set(),
      hiddenConfirmations: new Set(),
    },
    nodeScale: 1,
    graphSpacingDraft: 1,
  }
}

export interface GeneralConfig {
  autostart: boolean
  closeBehavior: CloseBehavior
}

/**
 * Per-preset saved fields. Each entry survives turning the preset off
 * and coming back — users don't have to re-enter an API key when they
 * briefly switch to a different provider.
 */
export interface ProviderOverride {
  apiKey?: string
  model?: string
  baseUrl?: string           // customEndpoint for custom presets, ollamaUrl for ollama
  azureApiVersion?: string
  azureModelFamily?: AzureModelFamily
  apiMode?: CustomApiMode
  maxContextSize?: number
  reasoning?: ReasoningConfig
  /** Reasoning used by structured ingest calls; defaults to off. */
  ingestReasoning?: ReasoningConfig
  localCliIsolation?: boolean
  codexCliTimeoutMinutes?: number
  codexReasoningEffort?: CodexReasoningEffort
  requestTimeoutMinutes?: number
  streamingEnabled?: boolean
  customHeaders?: Record<string, string>
}

export type ProviderConfigs = Record<string, ProviderOverride>

export interface CustomLlmPreset {
  id: string
  label: string
}

export interface TaskModelRoutingConfig {
  /** Null keeps chat on the globally active provider preset. */
  chatPresetId: string | null
  /** Null keeps ingest on the globally active provider preset. */
  ingestPresetId: string | null
}

interface WikiState {
  activeView:
    | "chat" | "search" | "graph" | "skills" | "settings"
    // Document base screens.
    | "home" | "origins" | "documents"
    // What needs a person, and the screens made to resolve it.
    | "attention" | "similar" | "document-types"
    // Who and what the documents name (the knowledge layer).
    | "entities" | "identity-doubts" | "matters"
    // The templates the documents share (D-466).
    | "templates"
    // The judicial portals (P-248).
    | "portals"
    // The changes other programs asked for, reached from Atención (D-495).
    | "program-requests"
  llmConfig: LlmConfig
  /** The folder the chat's agent works in (`<app data>/agent`), once known. */
  agentWorkspace: string | null
  /** Per-provider-preset stored overrides (API key, model, endpoint, …). */
  providerConfigs: ProviderConfigs
  customLlmPresets: CustomLlmPreset[]
  /** Which preset is currently active. `null` = no LLM configured. */
  activePresetId: string | null
  taskModelRouting: TaskModelRoutingConfig
  searchApiConfig: SearchApiConfig
  outputLanguage: OutputLanguage
  proxyConfig: ProxyConfig
  generalConfig: GeneralConfig
  graphUiState: GraphUiState
  dataVersion: number

  setActiveView: (view: WikiState["activeView"]) => void
  setLlmConfig: (config: LlmConfig) => void
  setAgentWorkspace: (path: string | null) => void
  setProviderConfigs: (configs: ProviderConfigs) => void
  setCustomLlmPresets: (presets: CustomLlmPreset[]) => void
  setActivePresetId: (id: string | null) => void
  setTaskModelRouting: (config: TaskModelRoutingConfig) => void
  setSearchApiConfig: (config: SearchApiConfig) => void
  setOutputLanguage: (lang: OutputLanguage) => void
  setProxyConfig: (config: ProxyConfig) => void
  setGeneralConfig: (config: GeneralConfig) => void
  setGraphUiState: (state: GraphUiState | ((current: GraphUiState) => GraphUiState)) => void
  resetGraphUiState: () => void
  bumpDataVersion: () => void
}

export const useWikiStore = create<WikiState>((set) => ({
  activeView: "home",
  llmConfig: {
    provider: "openai",
    apiKey: "",
    maxContextSize: 204800,
    model: "",
    ollamaUrl: "http://localhost:11434",
    customEndpoint: "",
    azureApiVersion: "2024-10-21",
    reasoning: { mode: "auto" },
    localCliIsolation: false,
  },
  agentWorkspace: null,
  providerConfigs: {},
  customLlmPresets: [],
  activePresetId: null,
  taskModelRouting: {
    chatPresetId: null,
    ingestPresetId: null,
  },
  dataVersion: 0,

  setActiveView: (activeView) => set({ activeView }),
  searchApiConfig: {
    provider: "none",
    apiKey: "",
    serpApiEngine: "google",
    searXngUrl: "",
    searXngCategories: ["general"],
    providerConfigs: {},
    deepResearchSource: "web",
    anyTxt: {
      enabled: false,
      endpoint: "http://127.0.0.1:9920",
      filterDir: "",
      filterExt: "*",
      limit: 20,
    },
  },

  outputLanguage: "auto",

  proxyConfig: {
    enabled: false,
    url: "",
    bypassLocal: true,
    acceptInvalidCerts: false,
  },

  generalConfig: {
    autostart: false,
    closeBehavior: "minimize",
  },

  graphUiState: createDefaultGraphUiState(),

  setLlmConfig: (llmConfig) => set({ llmConfig }),
  setAgentWorkspace: (agentWorkspace) => set({ agentWorkspace }),
  setProviderConfigs: (providerConfigs) => set({ providerConfigs }),
  setCustomLlmPresets: (customLlmPresets) => set({ customLlmPresets }),
  setActivePresetId: (activePresetId) => set({ activePresetId }),
  setTaskModelRouting: (taskModelRouting) => set({ taskModelRouting }),
  setSearchApiConfig: (searchApiConfig) => set({ searchApiConfig }),
  setOutputLanguage: (outputLanguage) => set({ outputLanguage }),
  setProxyConfig: (proxyConfig) => set({ proxyConfig }),
  setGeneralConfig: (generalConfig) => set({ generalConfig }),
  setGraphUiState: (graphUiState) =>
    set((state) => ({
      graphUiState: typeof graphUiState === "function"
        ? graphUiState(state.graphUiState)
        : graphUiState,
    })),
  resetGraphUiState: () => set({ graphUiState: createDefaultGraphUiState() }),
  bumpDataVersion: () => set((state) => ({ dataVersion: state.dataVersion + 1 })),
}))

export type { WikiState, LlmConfig, SearchApiConfig, OutputLanguage, ProxyConfig }
