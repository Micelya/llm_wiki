import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))

import type { ChatDocument } from "@/lib/chat-documents"
import { ChatInput } from "./chat-input"
import { DocumentChip, DocumentOptions, DocumentPicker } from "./chat-document-picker"

// Invented documents (R-24).
const lease: ChatDocument = { locationId: 7, name: "contrato de prueba.pdf", place: "Carpeta de prueba › Casos" }
const note: ChatDocument = { locationId: 8, name: "nota de prueba.docx", place: "Carpeta de prueba" }

function text(html: string): string {
  return html.replace(/<[^>]+>/g, " ").replace(/&quot;/g, '"').replace(/\s+/g, " ").trim()
}

describe("attaching documents of the base in the chat (P-243)", () => {
  it("shows each document found with its name and where it is", () => {
    const shown = text(renderToStaticMarkup(<DocumentOptions options={[lease, note]} activeIndex={1} onPick={() => {}} />))
    expect(shown).toBe("contrato de prueba.pdf Carpeta de prueba › Casos nota de prueba.docx Carpeta de prueba")
  })

  it("has a search box that says what it looks for, and when nothing matches", () => {
    const empty = text(
      renderToStaticMarkup(<DocumentPicker query="nada" options={[]} onQuery={() => {}} onPick={() => {}} onClose={() => {}} />),
    )
    expect(empty).toContain("Adjuntar un documento de la base: el chat lee su texto y lo cita.")
    expect(empty).toContain("Ningún documento con texto coincide con lo que escribiste.")
    const html = renderToStaticMarkup(<DocumentPicker query="" options={null} onQuery={() => {}} onPick={() => {}} onClose={() => {}} />)
    expect(html).toContain('placeholder="Buscá por el nombre, la carpeta o lo que dice"')
    expect(text(html)).toContain("Buscando…")
  })

  it("shows the documents attached to the message, each one to take out, and the button to attach more", () => {
    const html = renderToStaticMarkup(
      <ChatInput
        onSend={() => {}}
        onStop={() => {}}
        isStreaming={false}
        useWebSearch={false}
        useAnyTxtSearch={false}
        agentMode="standard"
        retrievalMode="standard"
        availableSkills={[]}
        selectedSkills={[]}
        searchDocuments={async () => []}
        selectedDocuments={[lease]}
        onSelectedDocumentsChange={() => {}}
        selectedContextFiles={[]}
        onUseWebSearchChange={() => {}}
        onUseAnyTxtSearchChange={() => {}}
        onAgentModeChange={() => {}}
        onRetrievalModeChange={() => {}}
        onSelectedSkillsChange={() => {}}
        onSelectedContextFilesChange={() => {}}
      />,
    )
    expect(html).toContain('title="contrato de prueba.pdf · Carpeta de prueba › Casos"')
    expect(html).toContain('title="Quitar «contrato de prueba.pdf» del mensaje"')
    expect(html).toMatch(/<button[^>]*title="Adjuntar un documento de la base[^"]*"[^>]*>.*?Documento<\/span><\/button>/)
    const chip = renderToStaticMarkup(<DocumentChip document={note} />)
    expect(chip).not.toContain("Quitar")
  })
})
