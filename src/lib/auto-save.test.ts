/**
 * The chat is written only after it was read at startup (P-239): the
 * store starts empty, and writing it before the saved history arrives
 * would replace the history with nothing.
 */
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest"

const { saveChatHistory, saveChatPreferences } = vi.hoisted(() => ({
  saveChatHistory: vi.fn().mockResolvedValue(undefined),
  saveChatPreferences: vi.fn().mockResolvedValue(undefined),
}))

vi.mock("./persist", () => ({ saveChatHistory, saveChatPreferences }))

async function fresh() {
  vi.resetModules()
  const autoSave = await import("./auto-save")
  const { useChatStore } = await import("@/stores/chat-store")
  autoSave.setupAutoSave()
  return { ...autoSave, useChatStore }
}

const conversation = { id: "c1", title: "Una consulta", createdAt: 1, updatedAt: 1 }

beforeEach(() => {
  saveChatHistory.mockClear()
  saveChatPreferences.mockClear()
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

describe("chat auto-save", () => {
  it("writes nothing before startup read the chat", async () => {
    const { useChatStore } = await fresh()
    useChatStore.setState({ conversations: [], messages: [] })
    await vi.advanceTimersByTimeAsync(5000)
    expect(saveChatHistory).not.toHaveBeenCalled()
    expect(saveChatPreferences).not.toHaveBeenCalled()
  })

  it("writes the history once it was read", async () => {
    const { useChatStore, resumeAutoSave } = await fresh()
    resumeAutoSave({ history: true })
    useChatStore.setState({ conversations: [conversation], messages: [] })
    await vi.advanceTimersByTimeAsync(2500)
    expect(saveChatHistory).toHaveBeenCalledWith([conversation], [])
    expect(saveChatPreferences).toHaveBeenCalled()
  })

  it("never replaces a history it could not read", async () => {
    const { useChatStore, resumeAutoSave } = await fresh()
    resumeAutoSave({ history: false })
    useChatStore.setState({ conversations: [], messages: [] })
    await vi.advanceTimersByTimeAsync(2500)
    expect(saveChatHistory).not.toHaveBeenCalled()
    expect(saveChatPreferences).toHaveBeenCalled()
  })

  it("waits while a reply is streaming", async () => {
    const { useChatStore, resumeAutoSave } = await fresh()
    resumeAutoSave({ history: true })
    useChatStore.setState({ conversations: [conversation], isStreaming: true })
    await vi.advanceTimersByTimeAsync(2500)
    expect(saveChatHistory).not.toHaveBeenCalled()
    useChatStore.setState({ isStreaming: false })
    await vi.advanceTimersByTimeAsync(2500)
    expect(saveChatHistory).toHaveBeenCalledTimes(1)
  })
})
