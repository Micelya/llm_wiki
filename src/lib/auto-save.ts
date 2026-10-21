import { useChatStore, type ChatState } from "@/stores/chat-store"
import { saveChatHistory, saveChatPreferences } from "./persist"

let chatTimer: ReturnType<typeof setTimeout> | null = null

/**
 * Nothing is written until the chat was read at startup (P-239): the
 * store starts empty, and writing it before the saved history arrives
 * would replace the history with nothing. The preferences are written
 * once startup is over; the history only if it could be read.
 */
let started = false
let historyRead = false

/** Starts writing the chat; `history` says whether the history was read. */
export function resumeAutoSave(options: { history: boolean }): void {
  started = true
  historyRead = options.history
}

function save(state: ChatState): Promise<unknown> {
  return Promise.allSettled([
    saveChatPreferences({
      useWebSearch: state.useWebSearch,
      useAnyTxtSearch: state.useAnyTxtSearch,
      agentMode: state.agentMode,
      retrievalMode: state.retrievalMode,
      selectedSkills: state.selectedSkills,
      disabledSkills: state.disabledSkills,
    }),
    historyRead ? saveChatHistory(state.conversations, state.messages) : Promise.resolve(),
  ])
}

/** Saves the chat's conversations and preferences, debounced 2s and never while a reply streams. */
export function setupAutoSave(): void {
  useChatStore.subscribe((state) => {
    if (!started || state.isStreaming) return
    if (chatTimer) clearTimeout(chatTimer)
    chatTimer = setTimeout(() => {
      save(state).catch(() => {})
    }, 2000)
  })
}
