import { describe, expect, it } from "vitest"
import type { OperationCard } from "@/commands/operations"
import { OPERATIONS_SOURCE, programRequestNotices, requesterOf } from "./operations"

// Every name is made up (R-24).
function request(id: number, title: string, engine: string | null): OperationCard {
  return {
    id,
    operation: "caso_agregar_documentos",
    title,
    request: null,
    target: null,
    items: [],
    warnings: [],
    note: null,
    secondConfirmation: false,
    proposedBy: { caller: "program:prg-1a2b3c4d5e", engine },
    state: "proposed",
    fingerprint: "abc",
    blocked: null,
    result: null,
    undoable: true,
    undoNote: null,
    open: null,
    input: {},
  }
}

describe("what the programs connected to Micelya raise in Atención", () => {
  it("nothing while no program asked for a change", () => {
    expect(programRequestNotices(null)).toEqual([])
    expect(programRequestNotices([])).toEqual([])
  })

  it("one decision with every request, who asked for it, and the way to its cards", () => {
    const [notice] = programRequestNotices([
      request(12, "Agregar 2 documentos al caso «Prueba c/ Ejemplo S.A.»", "Asistente de prueba"),
      request(13, "Cerrar el caso «Prueba alquiler»", null),
    ])
    expect(notice.id).toBe(`${OPERATIONS_SOURCE}:program-requests`)
    expect([notice.kind, notice.count]).toEqual(["decision", 2])
    expect(notice.title).toBe("2 pedidos de otros programas esperan tu confirmación")
    expect(notice.explanation).toContain("No se hizo nada")
    expect(notice.resolvedWhen).toContain("vence a los 7 días")
    expect(notice.actions[0]).toMatchObject({ kind: "go", label: "Revisar", target: { view: "program-requests" } })
    expect(notice.cases?.map((c) => [c.title, c.detail])).toEqual([
      ["Agregar 2 documentos al caso «Prueba c/ Ejemplo S.A.»", "Lo pidió «Asistente de prueba»"],
      ["Cerrar el caso «Prueba alquiler»", "Lo pidió «prg-1a2b3c4d5e»"],
    ])
    expect(requesterOf(request(1, "x", "Script de prueba"))).toBe("Script de prueba")
  })

  it("one request is said in the singular", () => {
    const [notice] = programRequestNotices([request(12, "Renombrar el caso", "Asistente de prueba")])
    expect(notice.title).toBe("1 pedido de otro programa espera tu confirmación")
  })
})
