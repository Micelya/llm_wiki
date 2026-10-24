import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it, vi } from "vitest"
import { operationHistoryLine, type OperationCard } from "@/commands/operations"
import { useChatStore } from "@/stores/chat-store"
import { OperationCardView, proposedByText } from "./operation-card"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(() => Promise.reject(new Error("sin app"))) }))

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/\s+/g, " ")
    .trim()
}

// Every name is made up (R-24).
function card(overrides: Partial<OperationCard> = {}): OperationCard {
  return {
    id: 12,
    operation: "caso_agregar_documentos",
    title: "Agregar 2 documentos al caso «Prueba c/ Ejemplo S.A.»",
    request: "sumá las dos cartas documento de marzo al caso Prueba",
    target: { kind: "caso", named: "Prueba", key: "mat-p-1a2b", name: "Prueba c/ Ejemplo S.A.", status: "ok", confirmation: "proposed", statusWord: "abierto" },
    items: [
      { named: "doc-41", key: "doc-41", name: "Carta documento 03-12.pdf", place: "Estudio › Prueba › Correspondencia", locationId: 88, date: null, status: "ok", checked: true, required: false, reason: null },
      { named: "doc-43", key: "doc-43", name: "Carta documento 03-19.pdf", place: "Estudio › Prueba › Correspondencia", locationId: 89, date: null, status: "ok", checked: true, required: false, reason: null },
      { named: "doc-57", key: "doc-57", name: "Recibo 03-20.pdf", place: "Estudio › Prueba", locationId: 90, date: null, status: "unchanged", checked: false, required: false, reason: "ya está en el caso" },
      { named: "la carta del 25 de marzo", key: null, name: null, place: null, locationId: null, date: null, status: "not_found", checked: false, required: false, reason: null },
      {
        named: "contrato",
        key: null,
        name: null,
        place: null,
        locationId: null,
        date: null,
        status: "choose",
        checked: false,
        required: false,
        reason: null,
        candidates: [
          { key: "doc-60", name: "Contrato de locación.pdf", place: "Estudio › Prueba", date: "1 de marzo" },
          { key: "doc-61", name: "Contrato de locación (copia).pdf", place: "Estudio › Otros" },
        ],
      },
    ],
    warnings: ["El caso era una propuesta: al confirmar, queda confirmado."],
    note: null,
    secondConfirmation: false,
    proposedBy: { caller: "chat", engine: "codex-cli:gpt-6-luna@high" },
    state: "proposed",
    fingerprint: "abc",
    blocked: null,
    result: null,
    undoable: true,
    undoNote: null,
    open: { kind: "caso", id: "mat-p-1a2b" },
    input: {},
    ...overrides,
  }
}

describe("the card of a change the chat proposed", () => {
  it("shows what it will do, on what, what was not found, which to choose and who proposed it", () => {
    const shown = text(renderToStaticMarkup(<OperationCardView card={card()} />))
    expect(shown).toContain("Agregar 2 documentos al caso «Prueba c/ Ejemplo S.A.»")
    expect(shown).toContain("Lo pediste: «sumá las dos cartas documento de marzo al caso Prueba»")
    expect(shown).toContain("Carta documento 03-12.pdf · Estudio › Prueba › Correspondencia")
    expect(shown).toContain("Recibo 03-20.pdf · Estudio › Prueba · ya está en el caso")
    expect(shown).toContain("«la carta del 25 de marzo»: no lo encontré en la base")
    expect(shown).toContain("«contrato»: coincide con varios, ¿cuál?")
    expect(shown).toContain("Contrato de locación.pdf · Estudio › Prueba · 1 de marzo")
    expect(shown).toContain("El caso era una propuesta: al confirmar, queda confirmado.")
    expect(shown).toContain("Confirmar")
    expect(shown).toContain("Cancelar")
    expect(shown).toContain("Lo propuso el modelo (gpt-6-luna@high). Todavía no se hizo nada.")
    expect(shown).not.toContain("Deshacer")
  })

  it("a change another program asked for says the program by its name, without a request of the person", () => {
    const shown = text(
      renderToStaticMarkup(<OperationCardView card={card({ request: null, proposedBy: { caller: "program:prg-1a2b3c4d5e", engine: "Asistente de prueba" } })} />),
    )
    expect(shown).toContain("Lo pidió «Asistente de prueba», un programa conectado. Todavía no se hizo nada.")
    expect(shown).not.toContain("Lo pediste")
    expect(proposedByText(card({ proposedBy: { caller: "program:prg-1a2b3c4d5e", engine: null } }))).toBe("Lo pidió «prg-1a2b3c4d5e», un programa conectado.")
    expect(proposedByText(card({ proposedBy: { caller: "screen", engine: null } }))).toBe("Desde la pantalla.")
  })

  it("folds a long list and blocks what cannot be confirmed", () => {
    const many = Array.from({ length: 14 }, (_, n) => ({
      named: `doc-${n}`, key: `doc-${n}`, name: `Factura ${n}.pdf`, place: "Estudio", locationId: n, date: null,
      status: "ok" as const, checked: true, required: false, reason: null,
    }))
    const shown = text(renderToStaticMarkup(<OperationCardView card={card({ items: many, blocked: "Elegí cuál es el caso." })} />))
    expect(shown).toContain("y 4 más, ver todos")
    expect(shown).not.toContain("Factura 12.pdf")
    expect(shown).toContain("Elegí cuál es el caso.")
    const html = renderToStaticMarkup(<OperationCardView card={card({ items: many, blocked: "Elegí cuál es el caso." })} />)
    expect(html).toMatch(/<button[^>]*disabled=""[^>]*>.*?Confirmar/)
  })

  it("once done says so and offers to undo it and open the case", () => {
    const shown = text(renderToStaticMarkup(
      <OperationCardView card={card({ state: "applied", result: "Hecho: 2 documentos agregados al caso «Prueba c/ Ejemplo S.A.»." })} />,
    ))
    expect(shown).toContain("Hecho: 2 documentos agregados al caso «Prueba c/ Ejemplo S.A.».")
    expect(shown).toContain("Deshacer")
    expect(shown).toContain("Abrir el caso")
    expect(shown).not.toContain("Confirmar")
    expect(shown).not.toContain("Todavía no se hizo nada")
  })

  it("says what became of it in the history of the next message", () => {
    expect(operationHistoryLine(card({ state: "applied" }))).toBe("[Acción] Agregar 2 documentos al caso «Prueba c/ Ejemplo S.A.»: hecha (la confirmó la persona)")
    expect(operationHistoryLine(card({ state: "cancelled" }))).toContain("cancelada")
    expect(operationHistoryLine(card({ state: "undone" }))).toContain("deshecha")
  })

  it("travels with the answer in the chat and is kept as the person leaves it", () => {
    const store = useChatStore.getState()
    const conversation = store.createConversation()
    store.finalizeStreamForConversation(conversation, "Te preparé la tarjeta.", [], [], undefined, [], {
      operationCard: card(),
      budget: { used: 2, max: 8, reached: false },
    })
    const message = useChatStore.getState().messages.find((m) => m.conversationId === conversation && m.role === "assistant")!
    expect(message.operationCard?.id).toBe(12)
    expect(message.budget?.used).toBe(2)
    useChatStore.getState().setMessageOperationCard(message.id, card({ state: "applied" }))
    expect(useChatStore.getState().messages.find((m) => m.id === message.id)?.operationCard?.state).toBe("applied")
  })
})
