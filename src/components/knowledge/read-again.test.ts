import { describe, expect, it } from "vitest"
import { rereadConfirmMessage, rereadNotice } from "./read-again"

describe("reading again with the model", () => {
  it("says first how many documents and questions it takes", () => {
    const message = rereadConfirmMessage({ documents: 3, questions: 5, requested: 0, engine: null })
    expect(message).toContain("3 documentos")
    expect(message).toContain("5 preguntas al modelo")
    expect(message).toContain("sin usar las respuestas de antes")
    expect(message).not.toContain("ya estaba")
    expect(rereadConfirmMessage({ documents: 1, questions: 1, requested: 1, engine: null })).toContain("1 ya estaba pedido")
  })

  it("says what was asked", () => {
    expect(rereadNotice(0)).toBe("No hay documentos leídos para volver a leer.")
    expect(rereadNotice(2)).toContain("2 documentos pedidos")
  })
})
