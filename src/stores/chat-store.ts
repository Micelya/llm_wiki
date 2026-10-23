import { create } from "zustand"
import type { ChatMessage, ContentBlock } from "@/lib/llm-client"
import i18n from "@/i18n"
import type { ChatAgentFileChange, ChatAgentMode, ChatAgentStep, ChatRetrievalMode, ChatUserInputRequest } from "@/lib/chat-agent-types"
import type { ChatDocument } from "@/lib/chat-documents"
import type { AgentBudget, OperationCard } from "@/commands/operations"

/**
 * An image attached to a user message. Field names mirror the
 * `image` variant of `ContentBlock` (see llm-providers.ts) so
 * converting a DisplayMessage into a wire ContentBlock is a no-op
 * spread — no remapping, no `data:` framing here (the provider
 * translators own that).
 */
export interface MessageImage {
  mediaType: string
  dataBase64: string
}

export interface Conversation {
  id: string
  title: string
  createdAt: number
  updatedAt: number
  selectedSkills?: string[]
  contextFiles?: string[]
  /** Documents of the base attached to the conversation (P-243). */
  contextDocuments?: ChatDocument[]
}

export interface MessageReference {
  title: string
  path: string
  kind?: "wiki" | "external" | "workspace"
  source?: string
  url?: string
  snippet?: string
  graphRelations?: string[]
}

export interface DisplayMessage {
  id: string
  role: "user" | "assistant" | "system"
  content: string
  timestamp: number
  conversationId: string
  references?: MessageReference[]  // pages cited in this response, saved at creation time
  agentSteps?: ChatAgentStep[]  // agent tool calls and routing decisions saved with assistant replies
  agentFileChanges?: ChatAgentFileChange[]  // concrete project files changed by this Agent turn
  userInputRequest?: ChatUserInputRequest  // dynamic schema-driven form requested by backend Agent
  images?: MessageImage[]  // images attached to a user message (vision input)
  contextFiles?: string[]  // absolute project files explicitly attached to this user turn
  contextDocuments?: ChatDocument[]  // documents of the base attached to this user turn (P-243)
  operationCard?: OperationCard  // a change the chat proposed: the card a person confirms (D-498)
  budget?: AgentBudget  // the turns the agent used, and where «Seguir buscando» goes on from
}

/** What an answer of the chat's agent brings besides its text. */
export interface AssistantExtras {
  operationCard?: OperationCard
  budget?: AgentBudget
}

export interface ChatState {
  conversations: Conversation[]
  activeConversationId: string | null
  messages: DisplayMessage[]
  isStreaming: boolean
  streamingContent: string
  maxHistoryMessages: number
  useWebSearch: boolean
  useAnyTxtSearch: boolean
  agentMode: ChatAgentMode
  retrievalMode: ChatRetrievalMode
  selectedSkills: string[]
  selectedContextFiles: string[]
  /** Documents of the base attached to the open conversation (P-243). */
  selectedContextDocuments: ChatDocument[]
  disabledSkills: string[]

  // Conversation management
  createConversation: () => string
  deleteConversation: (id: string) => void
  setActiveConversation: (id: string | null) => void
  renameConversation: (id: string, title: string) => void

  // Message management
  addMessage: (role: DisplayMessage["role"], content: string, images?: MessageImage[]) => void
  addMessageToConversation: (conversationId: string, role: DisplayMessage["role"], content: string, images?: MessageImage[], contextFiles?: string[], contextDocuments?: ChatDocument[]) => void
  setMessages: (messages: DisplayMessage[]) => void
  setConversations: (conversations: Conversation[]) => void
  setStreaming: (streaming: boolean) => void
  appendStreamToken: (token: string) => void
  finalizeStream: (content: string, references?: MessageReference[], agentSteps?: ChatAgentStep[], userInputRequest?: ChatUserInputRequest, agentFileChanges?: ChatAgentFileChange[]) => void
  finalizeStreamForConversation: (conversationId: string, content: string, references?: MessageReference[], agentSteps?: ChatAgentStep[], userInputRequest?: ChatUserInputRequest, agentFileChanges?: ChatAgentFileChange[], extras?: AssistantExtras) => void
  /** The card of a message, as it is after a person confirmed, cancelled or undid it. */
  setMessageOperationCard: (messageId: string, card: OperationCard) => void
  clearMessages: () => void
  setMaxHistoryMessages: (n: number) => void
  setUseWebSearch: (enabled: boolean) => void
  setUseAnyTxtSearch: (enabled: boolean) => void
  setAgentMode: (mode: ChatAgentMode) => void
  setRetrievalMode: (mode: ChatRetrievalMode) => void
  setSelectedSkills: (skills: string[]) => void
  setSelectedContextFiles: (paths: string[]) => void
  setSelectedContextDocuments: (documents: ChatDocument[]) => void
  setDisabledSkills: (skills: string[]) => void
  removeLastAssistantMessage: () => void  // for regenerate: remove last assistant reply

  // Helpers
  getActiveMessages: () => DisplayMessage[]
}

let messageCounter = 0

function nextId(): string {
  messageCounter += 1
  return `msg_${Date.now()}_${messageCounter}_${Math.random().toString(36).slice(2, 8)}`
}

function generateConversationId(): string {
  return `conv_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`
}

export const useChatStore = create<ChatState>((set, get) => ({
  conversations: [],
  activeConversationId: null,
  messages: [],
  isStreaming: false,
  streamingContent: "",
  maxHistoryMessages: 10,
  useWebSearch: false,
  useAnyTxtSearch: false,
  agentMode: "standard",
  retrievalMode: "standard",
  selectedSkills: [],
  selectedContextFiles: [],
  selectedContextDocuments: [],
  disabledSkills: [],

  createConversation: () => {
    const id = generateConversationId()
    const now = Date.now()
    const newConversation: Conversation = {
      id,
      title: i18n.t("chat.newConversation"),
      createdAt: now,
      updatedAt: now,
      selectedSkills: [],
      contextFiles: [],
      contextDocuments: [],
    }
    set((state) => ({
      conversations: [newConversation, ...state.conversations],
      activeConversationId: id,
      isStreaming: false,
      streamingContent: "",
      selectedSkills: [],
      selectedContextFiles: [],
      selectedContextDocuments: [],
    }))
    return id
  },

  deleteConversation: (id) =>
    set((state) => {
      const remaining = state.conversations.filter((c) => c.id !== id)
      const newActiveId =
        state.activeConversationId === id
          ? (remaining[0]?.id ?? null)
          : state.activeConversationId
      return {
        conversations: remaining,
        messages: state.messages.filter((m) => m.conversationId !== id),
        activeConversationId: newActiveId,
        selectedSkills: remaining.find((conversation) => conversation.id === newActiveId)?.selectedSkills ?? [],
        selectedContextFiles: remaining.find((conversation) => conversation.id === newActiveId)?.contextFiles ?? [],
        selectedContextDocuments: remaining.find((conversation) => conversation.id === newActiveId)?.contextDocuments ?? [],
      }
    }),

  setActiveConversation: (id) =>
    set((state) => ({
      activeConversationId: id,
      streamingContent: "",
      selectedSkills: state.conversations.find((conversation) => conversation.id === id)?.selectedSkills ?? [],
      selectedContextFiles: state.conversations.find((conversation) => conversation.id === id)?.contextFiles ?? [],
      selectedContextDocuments: state.conversations.find((conversation) => conversation.id === id)?.contextDocuments ?? [],
    })),

  renameConversation: (id, title) =>
    set((state) => ({
      conversations: state.conversations.map((c) =>
        c.id === id ? { ...c, title, updatedAt: Date.now() } : c
      ),
    })),

  addMessage: (role, content, images) => {
    const activeConversationId = get().activeConversationId
    if (!activeConversationId) return
    get().addMessageToConversation(activeConversationId, role, content, images)
  },

  addMessageToConversation: (conversationId, role, content, images, contextFiles, contextDocuments) =>
    set((state) => {
      const { conversations } = state
      if (!conversations.some((conversation) => conversation.id === conversationId)) return state

      const newMessage: DisplayMessage = {
        id: nextId(),
        role,
        content,
        timestamp: Date.now(),
        conversationId,
        ...(images && images.length > 0 ? { images } : {}),
        ...(contextFiles && contextFiles.length > 0 ? { contextFiles } : {}),
        ...(contextDocuments && contextDocuments.length > 0 ? { contextDocuments } : {}),
      }

      // Auto-set title from first user message (first 50 chars)
      const convMessages = state.messages.filter(
        (m) => m.conversationId === conversationId && m.role === "user"
      )
      const updatedConversations =
        role === "user" && convMessages.length === 0
          ? conversations.map((c) =>
              c.id === conversationId
                ? {
                    ...c,
                    // Image-only first message has empty text; fall
                    // back to a generic title so the sidebar entry
                    // isn't blank.
                    title: content.slice(0, 50) || (images && images.length > 0 ? i18n.t("chat.imageMessage") : c.title),
                    updatedAt: Date.now(),
                  }
                : c
            )
          : conversations.map((c) =>
              c.id === conversationId
                ? { ...c, updatedAt: Date.now() }
                : c
            )

      return {
        messages: [...state.messages, newMessage],
        conversations: updatedConversations,
      }
    }),

  setMessages: (messages) => set({ messages }),

  setConversations: (conversations) =>
    set((state) => ({
      conversations,
      selectedSkills: conversations.find((conversation) => conversation.id === state.activeConversationId)?.selectedSkills ?? [],
      selectedContextFiles: conversations.find((conversation) => conversation.id === state.activeConversationId)?.contextFiles ?? [],
      selectedContextDocuments: conversations.find((conversation) => conversation.id === state.activeConversationId)?.contextDocuments ?? [],
    })),

  setStreaming: (isStreaming) => set((state) => ({
    isStreaming,
    // Each new run owns its own stream buffer. Without this reset, a newly
    // created conversation can briefly render tokens left by another
    // conversation until the next token arrives.
    ...(isStreaming ? { streamingContent: "" } : state.streamingContent ? {} : {}),
  })),

  appendStreamToken: (token) =>
    set((state) => ({
      streamingContent: state.streamingContent + token,
    })),

  finalizeStream: (content, references, agentSteps, userInputRequest, agentFileChanges) => {
    const activeConversationId = get().activeConversationId
    if (!activeConversationId) {
      set({
        isStreaming: false,
        streamingContent: "",
      })
      return
    }
    get().finalizeStreamForConversation(
      activeConversationId,
      content,
      references,
      agentSteps,
      userInputRequest,
      agentFileChanges,
    )
  },

  finalizeStreamForConversation: (conversationId, content, references, agentSteps, userInputRequest, agentFileChanges, extras) =>
    set((state) => {
      const { conversations } = state
      if (!conversations.some((conversation) => conversation.id === conversationId)) {
        return {
          isStreaming: false,
          streamingContent: "",
        }
      }

      const newMessage: DisplayMessage = {
        id: nextId(),
        role: "assistant" as const,
        content,
        timestamp: Date.now(),
        conversationId,
        references,
        agentSteps,
        ...(agentFileChanges && agentFileChanges.length > 0 ? { agentFileChanges } : {}),
        ...(userInputRequest ? { userInputRequest } : {}),
        ...(extras?.operationCard ? { operationCard: extras.operationCard } : {}),
        ...(extras?.budget ? { budget: extras.budget } : {}),
      }

      return {
        isStreaming: false,
        streamingContent: "",
        messages: [...state.messages, newMessage],
        conversations: conversations.map((c) =>
          c.id === conversationId
            ? { ...c, updatedAt: Date.now() }
            : c
        ),
      }
    }),



  setMessageOperationCard: (messageId, card) =>
    set((state) => ({
      messages: state.messages.map((message) => (message.id === messageId ? { ...message, operationCard: card } : message)),
    })),

  clearMessages: () =>
    set((state) => ({
      messages: state.messages.filter(
        (m) => m.conversationId !== state.activeConversationId
      ),
    })),

  setMaxHistoryMessages: (maxHistoryMessages) => set({ maxHistoryMessages }),

  setUseWebSearch: (useWebSearch) => set({ useWebSearch }),

  setUseAnyTxtSearch: (useAnyTxtSearch) => set({ useAnyTxtSearch }),

  setAgentMode: (agentMode) => set({ agentMode }),

  setRetrievalMode: (retrievalMode) => set({ retrievalMode }),

  setSelectedSkills: (selectedSkills) =>
    set((state) => ({
      selectedSkills,
      conversations: state.activeConversationId
        ? state.conversations.map((conversation) =>
            conversation.id === state.activeConversationId
              ? { ...conversation, selectedSkills }
              : conversation
          )
        : state.conversations,
    })),

  setSelectedContextFiles: (selectedContextFiles) =>
    set((state) => ({
      selectedContextFiles,
      conversations: state.activeConversationId
        ? state.conversations.map((conversation) =>
            conversation.id === state.activeConversationId
              ? { ...conversation, contextFiles: selectedContextFiles }
              : conversation
          )
        : state.conversations,
    })),

  setSelectedContextDocuments: (selectedContextDocuments) =>
    set((state) => ({
      selectedContextDocuments,
      conversations: state.activeConversationId
        ? state.conversations.map((conversation) =>
            conversation.id === state.activeConversationId
              ? { ...conversation, contextDocuments: selectedContextDocuments }
              : conversation
          )
        : state.conversations,
    })),

  setDisabledSkills: (disabledSkills) => set({ disabledSkills }),

  removeLastAssistantMessage: () =>
    set((state) => {
      const activeId = state.activeConversationId
      if (!activeId) return state
      const activeMessages = state.messages.filter((m) => m.conversationId === activeId)
      // Find last assistant message
      const lastAssistantIdx = [...activeMessages].reverse().findIndex((m) => m.role === "assistant")
      if (lastAssistantIdx === -1) return state
      const msgToRemove = activeMessages[activeMessages.length - 1 - lastAssistantIdx]
      return {
        messages: state.messages.filter((m) => m.conversationId !== activeId || m.id !== msgToRemove.id),
      }
    }),

  getActiveMessages: () => {
    const { messages, activeConversationId } = get()
    if (!activeConversationId) return []
    return messages.filter((m) => m.conversationId === activeConversationId)
  },
}))

export function chatMessagesToLLM(messages: DisplayMessage[]): ChatMessage[] {
  return messages.map((m) => {
    // No images → keep the legacy string shape. Providers and the
    // single-string fast paths in the translators stay unchanged,
    // and existing tests that assert `content: "..."` keep passing.
    if (!m.images || m.images.length === 0) {
      return { role: m.role, content: m.content }
    }
    // Images present → emit a ContentBlock[]. Text first (so the
    // model reads the prompt before the images), then one image
    // block per attachment. An empty text (image-only message)
    // still gets a text block — harmless, and keeps the shape
    // uniform.
    const blocks: ContentBlock[] = [
      { type: "text", text: m.content },
      ...m.images.map((img): ContentBlock => ({
        type: "image",
        mediaType: img.mediaType,
        dataBase64: img.dataBase64,
      })),
    ]
    return { role: m.role, content: blocks }
  })
}
