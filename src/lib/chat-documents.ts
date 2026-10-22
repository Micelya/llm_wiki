import { listDocuments, type DocumentRow } from "@/commands/docbase"
import { useChatStore } from "@/stores/chat-store"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"

/**
 * Documents of the base attached to a message of the chat (P-243, D-453).
 * A person chooses them with a search over the base (or by typing «@» in
 * the message), or from a document with «Preguntar al chat sobre este
 * documento». The agent reads their text from the base, never a file on
 * disk, with their name and where they are, and cites them.
 */
export interface ChatDocument {
  /** The file chosen; the agent reads the text of its document. */
  locationId: number
  name: string
  /** Where it is: "<origin> › <folder>", or the origin alone at its root. */
  place: string
}

/** The most documents one message carries (as the backend reads them). */
export const MAX_CHAT_DOCUMENTS = 8
/** How many documents the search offers at once. */
export const CHAT_DOCUMENT_OPTIONS = 20
/** The most of the text of each attached document, and of all together, handed to a CLI provider. */
export const CLI_DOCUMENT_CHARS = 20_000
export const CLI_DOCUMENTS_TOTAL_CHARS = 48_000

/** "<origin> › <folder>", or the origin alone at its root. */
export function documentPlace(row: Pick<DocumentRow, "originName" | "folder">): string {
  return row.folder ? `${row.originName} › ${row.folder}` : row.originName
}

export function chatDocumentOf(row: Pick<DocumentRow, "locationId" | "name" | "originName" | "folder">): ChatDocument {
  return { locationId: row.locationId, name: row.name, place: documentPlace(row) }
}

/** What the search offers: documents with their text in the base, not attached yet. */
export function documentOptions(rows: DocumentRow[], attached: ChatDocument[]): ChatDocument[] {
  const taken = new Set(attached.map((document) => document.locationId))
  return rows
    .filter((row) => row.state === "processed" && row.contentId !== null && !taken.has(row.locationId))
    .map(chatDocumentOf)
}

/** One more document attached, once, up to `MAX_CHAT_DOCUMENTS`. */
export function withDocument(attached: ChatDocument[], document: ChatDocument): ChatDocument[] {
  if (attached.some((item) => item.locationId === document.locationId)) return attached
  return [...attached, document].slice(0, MAX_CHAT_DOCUMENTS)
}

/**
 * Searches the base for documents to attach: by words of their name, their
 * folder or their text; with no words, the most recent ones. One row per
 * document, only those whose text the base has.
 */
export async function searchChatDocuments(query: string, attached: ChatDocument[] = []): Promise<ChatDocument[]> {
  const words = query.trim()
  const page = await listDocuments({
    query: words || undefined,
    state: "processed",
    onePerDocument: true,
    sort: "recent",
    limit: CHAT_DOCUMENT_OPTIONS + attached.length,
  })
  return documentOptions(page.rows, attached).slice(0, CHAT_DOCUMENT_OPTIONS)
}

function escapeAttribute(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
}

function escapeText(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
}

/** The marks the recognition of a scan leaves before each page it read. */
function withoutRecognitionMarks(text: string): string {
  return text
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("<!-- texto-reconocido"))
    .join("\n")
}

/**
 * The attached documents for a CLI provider (Codex CLI, Claude Code), which
 * answers only from the prompt this side assembles: each one with its name,
 * where it is and its text from the base, bounded, and the instruction to
 * cite them. `readText` reads the text of a document from the base.
 */
export async function attachedDocumentsContext(
  documents: ChatDocument[],
  readText: (locationId: number) => Promise<string | null>,
): Promise<string> {
  if (documents.length === 0) return ""
  const texts = await Promise.all(documents.map((document) => readText(document.locationId).catch(() => null)))
  let remaining = CLI_DOCUMENTS_TOTAL_CHARS
  const blocks = documents.map((document, index) => {
    const head = `<document name="${escapeAttribute(document.name)}" place="${escapeAttribute(document.place)}">`
    const text = escapeText(withoutRecognitionMarks(texts[index] ?? "").trim())
    let body: string
    if (!text) {
      body = "(This document has no text in the base yet: a scan not recognized, or a file not read yet.)"
    } else if (remaining <= 0) {
      body = "(Left out for lack of room.)"
    } else {
      const limit = Math.min(CLI_DOCUMENT_CHARS, remaining)
      body = text.length > limit ? `${text.slice(0, limit)}\n[The text goes on in the base.]` : text
      remaining -= Math.min(text.length, limit)
    }
    return `${head}\n${body}\n</document>`
  })
  return [
    "Documents of the base the user attached to this message. Answer from their text, and when you use one, cite it by its name and where it is (for example: «name» (place)):",
    ...blocks,
  ].join("\n\n")
}

/** Opens the screen of an attached document. */
export function openChatDocument(locationId: number): void {
  useWikiStore.getState().setActiveView("documents")
  useDocbaseStore.getState().openDocument(locationId)
}

/**
 * «Preguntar al chat sobre este documento»: opens the chat with the
 * document attached, in a new conversation unless the one open has no
 * message yet, so the question is about it alone.
 */
export function askChatAbout(document: ChatDocument): void {
  const chat = useChatStore.getState()
  const open = chat.activeConversationId
  const used = open !== null && chat.messages.some((message) => message.conversationId === open)
  if (open === null || used) chat.createConversation()
  useChatStore.getState().setSelectedContextDocuments([document])
  useWikiStore.getState().setActiveView("chat")
}
