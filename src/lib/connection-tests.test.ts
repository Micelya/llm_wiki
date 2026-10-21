import { beforeEach, describe, expect, it, vi } from "vitest"
import type { LlmConfig } from "@/stores/wiki-store"
import {
  LLM_PROVIDER_TEST_MAX_TOKENS,
  testLlmConnection,
  testLlmFunction,
} from "./connection-tests"
import { streamChat } from "@/lib/llm-client"


vi.mock("@/lib/llm-client", () => ({
  streamChat: vi.fn(),
}))

const streamChatMock = vi.mocked(streamChat)

const llmConfig: LlmConfig = {
  provider: "custom",
  apiKey: "",
  model: "test-model",
  ollamaUrl: "http://localhost:11434",
  customEndpoint: "http://localhost:1234/v1",
  maxContextSize: 4096,
  apiMode: "chat_completions",
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe("provider connection tests", () => {
  it("passes LLM connection when any content is streamed", async () => {
    streamChatMock.mockImplementationOnce(async (_cfg, _messages, callbacks) => {
      callbacks.onToken("OK")
      callbacks.onDone()
    })

    const result = await testLlmConnection(llmConfig)

    expect(result.ok).toBe(true)
    expect(result.message).toContain("Response: OK")
  })

  it("validates LLM functional output token", async () => {
    streamChatMock.mockImplementationOnce(async (_cfg, _messages, callbacks) => {
      callbacks.onToken("LLM_WIKI_TEST_OK")
      callbacks.onDone()
    })

    const result = await testLlmFunction(llmConfig)

    expect(result.ok).toBe(true)
    expect(streamChatMock).toHaveBeenCalledWith(
      llmConfig,
      expect.any(Array),
      expect.any(Object),
      undefined,
      { max_tokens: LLM_PROVIDER_TEST_MAX_TOKENS, reasoning: { mode: "auto" } },
    )
  })

  it("uses enough output budget for reasoning-heavy local models", async () => {
    streamChatMock.mockImplementationOnce(async (_cfg, _messages, callbacks) => {
      callbacks.onToken("OK")
      callbacks.onDone()
    })

    await testLlmConnection(llmConfig)

    expect(streamChatMock).toHaveBeenCalledWith(
      llmConfig,
      expect.any(Array),
      expect.any(Object),
      undefined,
      expect.objectContaining({ max_tokens: 512 }),
    )
  })

  it.each([
    ["Claude Code", "claude-code", "connection", false],
    ["Claude Code", "claude-code", "connection", undefined],
    ["Claude Code", "claude-code", "functional", false],
    ["Claude Code", "claude-code", "functional", undefined],
    ["Codex CLI", "codex-cli", "connection", false],
    ["Codex CLI", "codex-cli", "connection", undefined],
    ["Codex CLI", "codex-cli", "functional", false],
    ["Codex CLI", "codex-cli", "functional", undefined],
  ] as const)(
    "preserves local CLI isolation during %s %s tests",
    async (_label, provider, kind, initialIsolation) => {
      streamChatMock.mockImplementationOnce(async (_cfg, _messages, callbacks) => {
        callbacks.onToken(kind === "functional" ? "LLM_WIKI_TEST_OK" : "OK")
        callbacks.onDone()
      })
      const cfg: LlmConfig = {
        ...llmConfig,
        provider,
        ...(initialIsolation === undefined ? {} : { localCliIsolation: initialIsolation }),
      }

      const result = kind === "functional"
        ? await testLlmFunction(cfg)
        : await testLlmConnection(cfg)

      expect(result.ok).toBe(true)
      expect(streamChatMock).toHaveBeenCalledWith(
        cfg,
        expect.any(Array),
        expect.any(Object),
        undefined,
        { max_tokens: LLM_PROVIDER_TEST_MAX_TOKENS, reasoning: { mode: "auto" } },
      )
      expect(cfg.localCliIsolation).toBe(initialIsolation)
    },
  )

  it("does not isolate non-CLI provider connection tests", async () => {
    streamChatMock.mockImplementationOnce(async (_cfg, _messages, callbacks) => {
      callbacks.onToken("OK")
      callbacks.onDone()
    })

    const result = await testLlmConnection(llmConfig)

    expect(result.ok).toBe(true)
    expect(streamChatMock).toHaveBeenCalledWith(
      llmConfig,
      expect.any(Array),
      expect.any(Object),
      undefined,
      { max_tokens: LLM_PROVIDER_TEST_MAX_TOKENS, reasoning: { mode: "auto" } },
    )
  })
})
