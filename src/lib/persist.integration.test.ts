/**
 * Tier 4 — real-FS integration tests for persistence.
 *
 * Tests exercise saveChatHistory / loadChatHistory against a REAL filesystem (temp dir per test). Mocks only
 * the Tauri invoke boundary — Node fs is the real deal. This catches bugs
 * that memory mocks can't: Unicode path handling, JSON escape round-trip,
 * directory auto-creation, legacy-format fallback.
 */
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest"
import { realFs, createTempProject, readFileRaw, writeFileRaw, fileExists } from "@/test-helpers/fs-temp"
import type { Conversation, DisplayMessage } from "@/stores/chat-store"

vi.mock("@/commands/fs", () => realFs)
// The chat lives in the app's data folder (D-415); here, a folder of the temp dir.
vi.mock("@/commands/docbase", () => ({
  getChatFolder: () => Promise.resolve(`${tmp.path}/chat`),
}))

import {
  saveChatHistory,
  loadChatHistory,
  saveChatPreferences,
  loadChatPreferences,
} from "./persist"

let tmp: { path: string; cleanup: () => Promise<void> }

beforeEach(async () => {
  tmp = await createTempProject("persist")
})

afterEach(async () => {
  await tmp.cleanup()
})

describe("chat persistence — round-trip (new format)", () => {
  function makeConv(id: string, title: string = "conv"): Conversation {
    return { id, title, createdAt: 0, updatedAt: 1 }
  }
  function makeMsg(id: string, convId: string, content: string): DisplayMessage {
    return { id, role: "user", content, timestamp: 0, conversationId: convId }
  }

  it("writes conversations.json + per-conversation chats/<id>.json", async () => {
    await saveChatHistory(
      [makeConv("c1"), makeConv("c2")],
      [
        makeMsg("m1", "c1", "hello"),
        makeMsg("m2", "c1", "world"),
        makeMsg("m3", "c2", "other"),
      ],
    )
    expect(await fileExists(`${tmp.path}/chat/conversations.json`)).toBe(true)
    expect(await fileExists(`${tmp.path}/chat/chats/c1.json`)).toBe(true)
    expect(await fileExists(`${tmp.path}/chat/chats/c2.json`)).toBe(true)
  })

  it("round-trips conversations + messages", async () => {
    const convs = [makeConv("c1", "Conv 1"), makeConv("c2", "Conv 2")]
    const msgs = [
      makeMsg("m1", "c1", "hi"),
      makeMsg("m2", "c2", "你好"),
    ]
    await saveChatHistory(convs, msgs)
    const loaded = await loadChatHistory()

    expect(loaded.conversations).toEqual(convs)
    // Messages may be returned in a different order (grouped by conv file),
    // so compare as sets.
    expect(loaded.messages).toEqual(expect.arrayContaining(msgs))
    expect(loaded.messages).toHaveLength(2)
  })

  it("does not persist base64 chat images into conversation JSON", async () => {
    const convs = [makeConv("c1", "Vision")]
    const msg: DisplayMessage = {
      ...makeMsg("m1", "c1", "what is this?"),
      images: [{ mediaType: "image/png", dataBase64: "A".repeat(1024) }],
    }

    await saveChatHistory(convs, [msg])
    expect(msg.images).toHaveLength(1)
    expect(msg.images?.[0].dataBase64).toBe("A".repeat(1024))

    const raw = await readFileRaw(`${tmp.path}/chat/chats/c1.json`)
    expect(raw).not.toContain("dataBase64")
    expect(raw).not.toContain("image/png")

    const loaded = await loadChatHistory()
    expect(loaded.messages[0]).toMatchObject({
      id: "m1",
      content: "what is this?",
    })
    expect(loaded.messages[0].images).toBeUndefined()
  })

  it("does not persist Agent rollback snapshots", async () => {
    const convs = [makeConv("c1", "Agent activity")]
    const msg: DisplayMessage = {
      ...makeMsg("m1", "c1", "done"),
      agentFileChanges: [{
        id: "run:file",
        path: `${tmp.path}/agent-workspace/file.md`,
        tool: "workspace.write_file",
        operation: "modified",
        additions: 1,
        deletions: 1,
        diff: "-before\n+after",
        timestamp: 1,
        beforeContent: "before",
        afterContent: "after",
      }],
    }

    await saveChatHistory(convs, [msg])
    const raw = await readFileRaw(`${tmp.path}/chat/chats/c1.json`)
    expect(raw).not.toContain("beforeContent")
    expect(raw).not.toContain("afterContent")
    expect(raw).toContain("-before\\n+after")
  })

  it("persists user-message context file attachments", async () => {
    const convs = [makeConv("c1", "Context files")]
    const msg: DisplayMessage = {
      ...makeMsg("m1", "c1", "summarize this"),
      contextFiles: [`${tmp.path}/wiki/overview.md`],
    }

    await saveChatHistory(convs, [msg])
    const loaded = await loadChatHistory()
    expect(loaded.messages[0].contextFiles).toEqual([
      `${tmp.path}/wiki/overview.md`,
    ])
  })

  it("caps each conversation's persisted messages at 100 (oldest dropped)", async () => {
    const convs = [makeConv("c1")]
    const msgs = Array.from({ length: 150 }, (_, i) =>
      makeMsg(`m${i}`, "c1", `msg ${i}`),
    )
    await saveChatHistory(convs, msgs)
    const loaded = await loadChatHistory()
    expect(loaded.messages).toHaveLength(100)
    // Should have kept the LAST 100 (m50 .. m149)
    expect(loaded.messages[0].id).toBe("m50")
    expect(loaded.messages[99].id).toBe("m149")
  })

  it("returns empty data when no persistence file exists", async () => {
    const loaded = await loadChatHistory()
    expect(loaded).toEqual({ conversations: [], messages: [] })
  })

  it("round-trips chat search preferences", async () => {
    await saveChatPreferences({
      useWebSearch: true,
      useAnyTxtSearch: false,
      agentMode: "deep",
      retrievalMode: "smart",
      selectedSkills: ["reviewer", "illustrator"],
      disabledSkills: ["legacy"],
    })
    await expect(loadChatPreferences()).resolves.toEqual({
      useWebSearch: true,
      useAnyTxtSearch: false,
      agentMode: "deep",
      retrievalMode: "smart",
      selectedSkills: ["reviewer", "illustrator"],
      disabledSkills: ["legacy"],
    })

    const raw = await readFileRaw(`${tmp.path}/chat/chat-preferences.json`)
    expect(raw).toContain('"useWebSearch": true')
    expect(raw).toContain('"retrievalMode": "smart"')
  })

  it("round-trips faithful source retrieval preference", async () => {
    await saveChatPreferences({
      useWebSearch: false,
      useAnyTxtSearch: false,
      agentMode: "standard",
      retrievalMode: "faithful",
      selectedSkills: [],
      disabledSkills: [],
    })

    const loaded = await loadChatPreferences()
    expect(loaded.retrievalMode).toBe("faithful")
  })

  it("defaults chat search preferences to off when no file exists", async () => {
    await expect(loadChatPreferences()).resolves.toEqual({
      useWebSearch: false,
      useAnyTxtSearch: false,
      agentMode: "standard",
      retrievalMode: "standard",
      selectedSkills: [],
      disabledSkills: [],
    })
  })

  it("skips missing per-conversation files without throwing", async () => {
    // conversations.json references c1 + c2, but chats/c2.json is missing
    await writeFileRaw(
      `${tmp.path}/chat/conversations.json`,
      JSON.stringify([makeConv("c1"), makeConv("c2")]),
    )
    await writeFileRaw(
      `${tmp.path}/chat/chats/c1.json`,
      JSON.stringify([makeMsg("m1", "c1", "hi")]),
    )
    const loaded = await loadChatHistory()
    expect(loaded.conversations).toHaveLength(2)
    expect(loaded.messages).toHaveLength(1)
  })

  it("recovers conversations from orphan chat files when conversation index was overwritten empty", async () => {
    await writeFileRaw(`${tmp.path}/chat/conversations.json`, "[]")
    await writeFileRaw(
      `${tmp.path}/chat/chats/c1.json`,
      JSON.stringify([
        makeMsg("m1", "c1", "First recovered question"),
        { ...makeMsg("m2", "c1", "answer"), role: "assistant" },
      ]),
    )
    await writeFileRaw(
      `${tmp.path}/chat/chats/c2.json`,
      JSON.stringify([makeMsg("m3", "c2", "Second recovered question")]),
    )

    const loaded = await loadChatHistory()

    expect(loaded.conversations.map((conversation) => conversation.id).sort()).toEqual(["c1", "c2"])
    expect(loaded.conversations.find((conversation) => conversation.id === "c1")?.title).toBe(
      "First recovered question",
    )
    expect(loaded.messages).toHaveLength(3)
  })

  it("recovers conversations from orphan chat files when conversation index is missing", async () => {
    await writeFileRaw(
      `${tmp.path}/chat/chats/c1.json`,
      JSON.stringify([makeMsg("m1", "c1", "Recovered without index")]),
    )

    const loaded = await loadChatHistory()

    expect(loaded.conversations).toHaveLength(1)
    expect(loaded.conversations[0].id).toBe("c1")
    expect(loaded.messages).toHaveLength(1)
  })

  it("preserves Unicode content through round-trip", async () => {
    const convs = [makeConv("c1", "中文对话 🎌")]
    const msgs = [
      makeMsg("m1", "c1", "你好，世界 🌍"),
      makeMsg("m2", "c1", "これはテスト"),
    ]
    await saveChatHistory(convs, msgs)
    const loaded = await loadChatHistory()
    expect(loaded.conversations[0].title).toBe("中文对话 🎌")
    expect(loaded.messages[0].content).toBe("你好，世界 🌍")
  })
})

describe("chat persistence — legacy format fallback", () => {
  function makeConv(id: string): Conversation {
    return { id, title: "t", createdAt: 0, updatedAt: 1 }
  }
  function makeMsg(id: string, convId: string): DisplayMessage {
    return { id, role: "user", content: "c", timestamp: 0, conversationId: convId }
  }

  it("falls back to chat-history.json flat-array format", async () => {
    // Very old format: flat array of messages
    const legacyMessages = [
      { id: "m1", role: "user", content: "old", timestamp: 100, conversationId: "ignored" },
      { id: "m2", role: "assistant", content: "older", timestamp: 200, conversationId: "ignored" },
    ]
    await writeFileRaw(
      `${tmp.path}/chat/chat-history.json`,
      JSON.stringify(legacyMessages),
    )

    const loaded = await loadChatHistory()
    expect(loaded.conversations).toHaveLength(1)
    expect(loaded.conversations[0].id).toBe("default")
    expect(loaded.messages).toHaveLength(2)
    expect(loaded.messages[0].conversationId).toBe("default")
  })

  it("falls back to chat-history.json combined-object format", async () => {
    const old = {
      conversations: [makeConv("c1")],
      messages: [makeMsg("m1", "c1")],
    }
    await writeFileRaw(
      `${tmp.path}/chat/chat-history.json`,
      JSON.stringify(old),
    )

    const loaded = await loadChatHistory()
    expect(loaded.conversations).toHaveLength(1)
    expect(loaded.messages).toHaveLength(1)
  })

  it("new format wins over legacy when both exist", async () => {
    await writeFileRaw(
      `${tmp.path}/chat/chat-history.json`,
      JSON.stringify({ conversations: [makeConv("legacy")], messages: [] }),
    )
    await writeFileRaw(
      `${tmp.path}/chat/conversations.json`,
      JSON.stringify([makeConv("new")]),
    )
    await writeFileRaw(`${tmp.path}/chat/chats/new.json`, "[]")

    const loaded = await loadChatHistory()
    expect(loaded.conversations[0].id).toBe("new")
  })
})

// Keep readFileRaw exported (referenced for a future direct-inspection test)
void readFileRaw
