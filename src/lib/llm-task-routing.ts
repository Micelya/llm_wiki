import { findLlmPreset } from "@/components/settings/llm-presets"
import { resolveConfig } from "@/components/settings/preset-resolver"
import type {
  LlmConfig,
  CustomLlmPreset,
  ProviderConfigs,
  TaskModelRoutingConfig,
} from "@/stores/wiki-store"
import { useWikiStore } from "@/stores/wiki-store"

export type LlmTaskKind = "chat" | "ingest"

/**
 * Resolve a task-specific provider from the current preset overrides.
 * Routing stores preset ids rather than credential snapshots so API-key,
 * endpoint, and model edits take effect immediately. Missing/deleted preset
 * ids fail safely to the active global configuration.
 */
export function resolveTaskLlmConfig(
  task: LlmTaskKind,
  fallback: LlmConfig,
  providerConfigs: ProviderConfigs,
  routing: TaskModelRoutingConfig,
  customPresets: CustomLlmPreset[] = [],
): LlmConfig {
  const presetId = task === "chat" ? routing.chatPresetId : routing.ingestPresetId
  if (!presetId) return fallback
  const preset = findLlmPreset(presetId, customPresets)
  if (!preset) return fallback
  return resolveConfig(preset, providerConfigs[presetId], fallback)
}

/** Resolve against one atomic Zustand snapshot for imperative/background code. */
export function getTaskLlmConfig(task: LlmTaskKind, fallback?: LlmConfig): LlmConfig {
  const state = useWikiStore.getState()
  return resolveTaskLlmConfig(
    task,
    fallback ?? state.llmConfig,
    state.providerConfigs,
    state.taskModelRouting,
    state.customLlmPresets,
  )
}
