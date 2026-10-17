import { describe, expect, it } from "vitest"
import { CODEX_REASONING_LEVELS, DEFAULT_CODEX_REASONING, codexReasoningEffort } from "./codex-reasoning"
import { resolveConfig } from "@/components/settings/preset-resolver"
import type { LlmPreset } from "@/components/settings/llm-presets"
import type { LlmConfig } from "@/stores/wiki-store"
import { useWikiStore } from "@/stores/wiki-store"
import { getAskConfig } from "./document-asking"

const fallback: LlmConfig = {
  provider: "openai",
  apiKey: "",
  model: "",
  ollamaUrl: "http://localhost:11434",
  customEndpoint: "",
  maxContextSize: 8192,
}

const codex: LlmPreset = { id: "codex-cli", label: "Codex CLI", provider: "codex-cli", defaultModel: "m" }
const claude: LlmPreset = { id: "claude-code-cli", label: "Claude Code CLI", provider: "claude-code", defaultModel: "sonnet" }

describe("Codex reasoning level", () => {
  it("offers only low, medium and high, and falls back to the default", () => {
    expect(CODEX_REASONING_LEVELS).toEqual(["low", "medium", "high"])
    expect(codexReasoningEffort("high")).toBe("high")
    for (const value of [undefined, "", "xhigh", "max", 3]) {
      expect(codexReasoningEffort(value)).toBe(DEFAULT_CODEX_REASONING)
    }
  })

  it("is kept for the Codex preset only", () => {
    expect(resolveConfig(codex, { codexReasoningEffort: "high" }, fallback).codexReasoningEffort).toBe("high")
    expect(resolveConfig(codex, {}, fallback).codexReasoningEffort).toBe(DEFAULT_CODEX_REASONING)
    expect(resolveConfig(claude, { codexReasoningEffort: "high" }, fallback).codexReasoningEffort).toBeUndefined()
  })

  it("goes with the engine that answers the questions about documents", () => {
    const before = useWikiStore.getState().llmConfig
    try {
      useWikiStore.setState({ llmConfig: { ...fallback, provider: "codex-cli", model: "m", codexReasoningEffort: "low" } })
      expect(getAskConfig()).toEqual({ engine: "codex-cli", model: "m", reasoning: "low" })
      useWikiStore.setState({ llmConfig: { ...fallback, provider: "codex-cli", model: "m" } })
      expect(getAskConfig()?.reasoning).toBe(DEFAULT_CODEX_REASONING)
    } finally {
      useWikiStore.setState({ llmConfig: before })
    }
  })
})
