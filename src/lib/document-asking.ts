/**
 * Which engine answers the base's questions about documents (their type,
 * their stage, their data).
 *
 * The questions happen in Rust, behind an engine-neutral contract, like
 * text recognition (`text-recognition.ts`); this module only says which
 * engine the configured model is, so callers never name one. Today only
 * the Codex CLI can answer from Rust.
 */
import type { AskConfig } from "@/commands/docbase"
import { useWikiStore } from "@/stores/wiki-store"
import { codexReasoningEffort } from "@/lib/codex-reasoning"

export function getAskConfig(): AskConfig | null {
  const { llmConfig } = useWikiStore.getState()
  if (llmConfig.provider !== "codex-cli") return null
  return { engine: "codex-cli", model: llmConfig.model, reasoning: codexReasoningEffort(llmConfig.codexReasoningEffort) }
}
