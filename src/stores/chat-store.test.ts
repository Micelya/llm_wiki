import { beforeEach, describe, expect, it } from "vitest"
import { useChatStore } from "./chat-store"

describe("chat-store conversation isolation", () => {
  beforeEach(() => {
    useChatStore.setState({
      conversations: [],
      activeConversationId: null,
      messages: [],
      isStreaming: false,
      streamingContent: "",
      useWebSearch: false,
      useAnyTxtSearch: false,
      agentMode: "standard",
      retrievalMode: "standard",
      selectedSkills: [],
      selectedContextFiles: [],
      selectedContextDocuments: [],
      disabledSkills: [],
    })
  })

  it("defaults to standard retrieval and allows explicit retrieval modes", () => {
    expect(useChatStore.getState().retrievalMode).toBe("standard")
    useChatStore.getState().setRetrievalMode("smart")
    expect(useChatStore.getState().retrievalMode).toBe("smart")
    useChatStore.getState().setRetrievalMode("faithful")
    expect(useChatStore.getState().retrievalMode).toBe("faithful")
  })

  it("writes async assistant results back to the original conversation", () => {
    const store = useChatStore.getState()
    const first = store.createConversation()
    store.addMessageToConversation(first, "user", "first question")

    const second = useChatStore.getState().createConversation()
    expect(useChatStore.getState().activeConversationId).toBe(second)

    useChatStore
      .getState()
      .finalizeStreamForConversation(first, "first answer")

    const state = useChatStore.getState()
    const firstMessages = state.messages.filter((message) => message.conversationId === first)
    const secondMessages = state.messages.filter((message) => message.conversationId === second)

    expect(firstMessages.map((message) => message.content)).toEqual([
      "first question",
      "first answer",
    ])
    expect(secondMessages).toEqual([])
  })

  it("clears stale stream content when a new stream starts", () => {
    useChatStore.setState({
      streamingContent: "old conversation tokens",
      isStreaming: false,
    })

    useChatStore.getState().setStreaming(true)

    expect(useChatStore.getState().streamingContent).toBe("")
    expect(useChatStore.getState().isStreaming).toBe(true)
  })

  it("creates globally unique message ids across conversations", () => {
    const first = useChatStore.getState().createConversation()
    useChatStore.getState().addMessageToConversation(first, "user", "first")

    const second = useChatStore.getState().createConversation()
    useChatStore.getState().addMessageToConversation(second, "user", "second")

    const ids = useChatStore.getState().messages.map((message) => message.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  it("removes the last assistant message only from the active conversation", () => {
    useChatStore.setState({
      conversations: [
        { id: "c1", title: "One", createdAt: 1, updatedAt: 1 },
        { id: "c2", title: "Two", createdAt: 2, updatedAt: 2 },
      ],
      activeConversationId: "c2",
      messages: [
        { id: "same", conversationId: "c1", role: "assistant", content: "keep", timestamp: 1 },
        { id: "same", conversationId: "c2", role: "assistant", content: "remove", timestamp: 2 },
      ],
    })

    useChatStore.getState().removeLastAssistantMessage()

    expect(useChatStore.getState().messages).toEqual([
      { id: "same", conversationId: "c1", role: "assistant", content: "keep", timestamp: 1 },
    ])
  })

  it("clears stale stream content when creating or switching conversations", () => {
    const first = useChatStore.getState().createConversation()
    useChatStore.setState({
      streamingContent: "old conversation tokens",
      isStreaming: true,
    })

    const second = useChatStore.getState().createConversation()

    expect(useChatStore.getState().activeConversationId).toBe(second)
    expect(useChatStore.getState().streamingContent).toBe("")
    expect(useChatStore.getState().isStreaming).toBe(false)

    useChatStore.setState({
      streamingContent: "more stale tokens",
      isStreaming: true,
    })
    useChatStore.getState().setActiveConversation(first)

    expect(useChatStore.getState().activeConversationId).toBe(first)
    expect(useChatStore.getState().streamingContent).toBe("")
  })

  it("stores selected skills per conversation and starts new conversations empty", () => {
    const first = useChatStore.getState().createConversation()
    useChatStore.getState().setSelectedSkills(["cover-image"])

    const second = useChatStore.getState().createConversation()

    expect(useChatStore.getState().activeConversationId).toBe(second)
    expect(useChatStore.getState().selectedSkills).toEqual([])

    useChatStore.getState().setSelectedSkills(["ppt"])
    useChatStore.getState().setActiveConversation(first)

    expect(useChatStore.getState().selectedSkills).toEqual(["cover-image"])

    useChatStore.getState().setActiveConversation(second)

    expect(useChatStore.getState().selectedSkills).toEqual(["ppt"])
  })

  it("stores selected context files per conversation and starts new conversations empty", () => {
    const first = useChatStore.getState().createConversation()
    useChatStore.getState().setSelectedContextFiles(["wiki/overview.md"])

    const second = useChatStore.getState().createConversation()
    expect(useChatStore.getState().selectedContextFiles).toEqual([])

    useChatStore.getState().setSelectedContextFiles(["raw/sources/source.txt"])
    useChatStore.getState().setActiveConversation(first)
    expect(useChatStore.getState().selectedContextFiles).toEqual(["wiki/overview.md"])

    useChatStore.getState().setActiveConversation(second)
    expect(useChatStore.getState().selectedContextFiles).toEqual(["raw/sources/source.txt"])
  })

  it("keeps the documents of the base attached to each conversation, and on the message sent (P-243)", () => {
    // Invented documents (R-24).
    const lease = { locationId: 7, name: "contrato de prueba.pdf", place: "Carpeta de prueba › Casos" }
    const note = { locationId: 8, name: "nota de prueba.docx", place: "Carpeta de prueba" }
    const first = useChatStore.getState().createConversation()
    useChatStore.getState().setSelectedContextDocuments([lease])
    const second = useChatStore.getState().createConversation()
    expect(useChatStore.getState().selectedContextDocuments).toEqual([])
    useChatStore.getState().setSelectedContextDocuments([note])
    useChatStore.getState().setActiveConversation(first)
    expect(useChatStore.getState().selectedContextDocuments).toEqual([lease])
    useChatStore.getState().addMessageToConversation(first, "user", "¿qué dice?", [], [], [lease])
    expect(useChatStore.getState().messages[0].contextDocuments).toEqual([lease])
    // They stay attached for the next questions of the conversation.
    expect(useChatStore.getState().selectedContextDocuments).toEqual([lease])
    useChatStore.getState().deleteConversation(first)
    expect(useChatStore.getState().activeConversationId).toBe(second)
    expect(useChatStore.getState().selectedContextDocuments).toEqual([note])
  })

  it("stores the context file snapshot on the user message", () => {
    const conversationId = useChatStore.getState().createConversation()
    useChatStore.getState().addMessageToConversation(
      conversationId,
      "user",
      "summarize this",
      [],
      ["/project/wiki/overview.md"],
    )

    expect(useChatStore.getState().messages[0].contextFiles).toEqual([
      "/project/wiki/overview.md",
    ])
  })
})
