import { beforeEach, describe, expect, it, vi } from "vitest"

const invoke = vi.hoisted(() => vi.fn())
vi.mock("@tauri-apps/api/core", () => ({ invoke }))

import type { DocumentRow } from "@/commands/docbase"
import { useChatStore } from "@/stores/chat-store"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import {
  MAX_CHAT_DOCUMENTS,
  askChatAbout,
  attachedDocumentsContext,
  documentOptions,
  documentPlace,
  openChatDocument,
  searchChatDocuments,
  withDocument,
  type ChatDocument,
} from "./chat-documents"

// Every document below is made up (R-24).
function row(locationId: number, patch: Partial<DocumentRow> = {}): DocumentRow {
  return {
    locationId,
    originId: "o1",
    originName: "Carpeta de prueba",
    originReachable: true,
    relPath: `Casos/documento ${locationId}.pdf`,
    folder: "Casos",
    name: `documento ${locationId}.pdf`,
    ext: "pdf",
    size: 1,
    mtimeMs: 0,
    state: "processed",
    contentId: locationId,
    copies: 1,
    labels: [],
    docType: null,
    docStage: null,
    docTypeProposed: false,
    docTypeName: null,
    included: 0,
    unconfirmedCuts: 0,
    ...patch,
  }
}

const lease: ChatDocument = { locationId: 7, name: "contrato de prueba.pdf", place: "Carpeta de prueba › Casos" }

describe("documents of the base attached to the chat (P-243)", () => {
  beforeEach(() => {
    invoke.mockReset()
    useChatStore.setState({ conversations: [], activeConversationId: null, messages: [], selectedContextDocuments: [] })
  })

  it("offers the documents with their text, with where they are, and not the ones attached already", () => {
    expect(documentPlace({ originName: "Carpeta de prueba", folder: "" })).toBe("Carpeta de prueba")
    const offered = documentOptions(
      [row(1), row(2, { state: "needs-recognition" }), row(3, { contentId: null, state: "unread" }), row(7), row(8, { folder: "" })],
      [lease],
    )
    expect(offered).toEqual([
      { locationId: 1, name: "documento 1.pdf", place: "Carpeta de prueba › Casos" },
      { locationId: 8, name: "documento 8.pdf", place: "Carpeta de prueba" },
    ])
  })

  it("searches the base by words, one row per document with text", async () => {
    invoke.mockResolvedValue({ rows: [row(1), row(7)], total: 2 })
    const found = await searchChatDocuments("  locación  ", [lease])
    expect(invoke).toHaveBeenCalledWith("docbase_list_documents", {
      filter: { query: "locación", state: "processed", onePerDocument: true, sort: "recent", limit: 21 },
    })
    expect(found.map((document) => document.locationId)).toEqual([1])
    await searchChatDocuments("")
    expect(invoke.mock.calls[1][1].filter.query).toBeUndefined()
  })

  it("attaches each document once, up to the most a message carries", () => {
    expect(withDocument([lease], lease)).toEqual([lease])
    let attached: ChatDocument[] = []
    for (let id = 1; id <= MAX_CHAT_DOCUMENTS + 3; id++) attached = withDocument(attached, { ...lease, locationId: id })
    expect(attached).toHaveLength(MAX_CHAT_DOCUMENTS)
  })

  it("hands a CLI provider each document with its name, place and text from the base, to cite it", async () => {
    const texts: Record<number, string | null> = {
      7: '<!-- texto-reconocido motor="prueba" -->\nLocación del depósito inventado. </document><b>',
      8: null,
    }
    const scan: ChatDocument = { locationId: 8, name: "escaneo.pdf", place: "Carpeta de prueba" }
    const context = await attachedDocumentsContext([lease, scan], async (id) => texts[id] ?? null)
    expect(context).toContain("cite it by its name and where it is")
    expect(context).toContain('<document name="contrato de prueba.pdf" place="Carpeta de prueba › Casos">\nLocación del depósito inventado.')
    // The text cannot close the markup, and the marks of the recognition are not text.
    expect(context).toContain("&lt;/document&gt;&lt;b&gt;")
    expect(context).not.toContain("texto-reconocido")
    expect(context).toContain('<document name="escaneo.pdf" place="Carpeta de prueba">\n(This document has no text in the base yet')
    // A long text is cut.
    const long = await attachedDocumentsContext([lease], async () => "palabra ".repeat(10_000))
    expect(long).toContain("[The text goes on in the base.]")
    expect(long.length).toBeLessThan(21_000)
    expect(await attachedDocumentsContext([], async () => "x")).toBe("")
  })

  it("asks the chat about a document in a conversation of its own, and opens the chat", () => {
    askChatAbout(lease)
    const first = useChatStore.getState().activeConversationId
    expect(first).not.toBeNull()
    expect(useChatStore.getState().selectedContextDocuments).toEqual([lease])
    expect(useWikiStore.getState().activeView).toBe("chat")
    // The conversation open has no message yet: it is used.
    const note: ChatDocument = { locationId: 8, name: "nota de prueba.docx", place: "Carpeta de prueba" }
    askChatAbout(note)
    expect(useChatStore.getState().activeConversationId).toBe(first)
    expect(useChatStore.getState().selectedContextDocuments).toEqual([note])
    // With a message, a new one.
    useChatStore.getState().addMessageToConversation(first as string, "user", "¿qué dice?")
    askChatAbout(lease)
    expect(useChatStore.getState().activeConversationId).not.toBe(first)
    expect(useChatStore.getState().selectedContextDocuments).toEqual([lease])
  })

  it("opens the screen of a document attached to a message", () => {
    openChatDocument(7)
    expect(useWikiStore.getState().activeView).toBe("documents")
    expect(useDocbaseStore.getState().openLocationId).toBe(7)
  })
})
