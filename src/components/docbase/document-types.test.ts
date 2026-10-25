import { describe, expect, it } from "vitest"
import type { DocumentType, SealVerdict, TypesPlan, TypesRunReport } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"
import {
  confidenceText,
  dataRows,
  questionsText,
  runConfirmMessage,
  runReportText,
  sealText,
  typeLabel,
  typeSourceText,
} from "./document-types"

const words = vocabularyFor("legal", "es")

function found(overrides: Partial<DocumentType> = {}): DocumentType {
  return {
    contentId: 1,
    docType: "carta-documento",
    stage: "vista-previa",
    proposed: false,
    proposedName: null,
    proposedDescription: null,
    source: "model",
    principal: 1,
    confidence: 0.87,
    reason: "Tiene el formulario del correo.",
    computedAt: 0,
    ...overrides,
  }
}

function plan(overrides: Partial<TypesPlan> = {}): TypesPlan {
  return { documents: 10, typed: 4, typeQuestions: 3, dataQuestions: 1, proposedTypes: [], engine: null, ...overrides }
}

describe("the type of a document in words", () => {
  it("names the type and the stage with the vocabulary", () => {
    expect(typeLabel("carta-documento", "vista-previa", false, null, words)).toBe("Carta documento · Vista previa")
    expect(typeLabel("factura", null, false, null, words)).toBe("Factura")
  })

  it("marks a type the model proposed and shows it by its proposed name", () => {
    expect(typeLabel("receta-de-cocina", null, true, "Receta de cocina", words)).toBe("Receta de cocina (propuesto)")
  })

  it("shows a type a person confirmed by the name it was given, without the mark", () => {
    expect(typeLabel("receta-de-cocina", null, false, "Receta", words)).toBe("Receta")
  })

  it("says where the type came from", () => {
    expect(typeSourceText(found({ source: "marks" }))).toContain("marcas comprobadas")
    expect(typeSourceText(found({ source: "family" }))).toContain("documento principal de su familia")
    expect(confidenceText(found())).toBe("Seguridad del modelo: 87 %")
    expect(confidenceText(found({ source: "marks", confidence: null }))).toBeNull()
  })

  it("groups the data by field, with their names", () => {
    const rows = dataRows(
      [
        { field: "facturas", value: "A0001-00000001", source: "rule" },
        { field: "facturas", value: "A0001-00000002", source: "rule" },
        { field: "numero-de-envio", value: "XY123456789AR", source: "model" },
      ],
      words,
    )
    expect(rows).toEqual([
      { field: "facturas", name: "Facturas", values: ["A0001-00000001", "A0001-00000002"], byModel: false },
      { field: "numero-de-envio", name: "Número de envío", values: ["XY123456789AR"], byModel: true },
    ])
  })
})

describe("the notice before asking the model", () => {
  it("counts the questions it will make", () => {
    expect(questionsText(plan())).toBe("3 preguntas sobre tipo y etapa y 1 pregunta sobre datos")
    expect(questionsText(plan({ typeQuestions: 0, dataQuestions: 0 }))).toBeNull()
  })

  it("says that it costs quota, and when the limit leaves some for later", () => {
    const message = runConfirmMessage(plan({ typeQuestions: 80 }), 60)
    expect(message).toContain("consume cupo")
    expect(message).toContain("como mucho 60")
    expect(runConfirmMessage(plan(), 60)).not.toContain("como mucho")
  })

  it("tells what a run did", () => {
    const report: TypesRunReport = {
      asked: 3,
      failed: 0,
      typed: 7,
      dataAsked: 1,
      dataFailed: 0,
      corrected: 2,
      left: 0,
      errors: [],
    }
    expect(runReportText(report)).toBe(
      "Se hicieron 4 preguntas al modelo. 7 documentos quedaron con tipo. 2 relaciones cambiaron por las reglas del tipo.",
    )
    expect(runReportText({ ...report, asked: 0, dataAsked: 0, typed: 0, corrected: 0 })).toBe(
      "No hizo falta preguntarle nada al modelo.",
    )
  })
})

describe("what the digital signatures of a file say", () => {
  const words = vocabularyFor("legal", "es")
  const seal = (fields: Partial<SealVerdict>): SealVerdict => ({ state: "sealed", laterValidation: false, failing: 0, ...fields })

  it("says who sealed it and when, in the vocabulary's words", () => {
    const text = sealText(seal({ serviceName: "Servicio de prueba", signedAt: "2026-09-07T15:00:00Z" }), words)
    expect(text).toBe("Íntegro: sellado por Servicio de prueba el 07/09/2026; no cambió desde entonces.")
    expect(sealText(seal({ serviceName: "Servicio de prueba" }), words)).toBe("Íntegro: sellado por Servicio de prueba; no cambió desde entonces.")
  })

  it("says that it does not verify, without technical words", () => {
    expect(sealText(seal({ state: "modified" }), words)).toBe("La firma digital no verifica: el archivo cambió después de firmarse.")
    expect(sealText(seal({ state: "invalid" }), words)).toBe("La firma digital no verifica.")
  })

  it("adds the validation added later and the other signatures that do not verify", () => {
    const text = sealText(seal({ state: "valid-unknown", laterValidation: true, failing: 3 }), words) ?? ""
    expect(text.startsWith("Íntegro: firmado digitalmente por alguien")).toBe(true)
    expect(text).toContain("validar la firma")
    expect(text).toContain(": 3.")
  })

  it("is nothing without signatures", () => {
    expect(sealText(null, words)).toBeNull()
    expect(sealText(seal({ state: "otro" }), words)).toBeNull()
  })
})
