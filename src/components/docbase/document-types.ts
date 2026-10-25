import type { Datum, DocumentRow, DocumentType, SealVerdict, TypesPlan, TypesRunReport } from "@/commands/docbase"
import { dataFieldName, documentTypeName, stageName, vocabulary, type Vocabulary } from "@/vocabulary"
import { formatDate, plural } from "./format"

/**
 * What the screens say about the type, stage and data of documents,
 * apart from the screens so it can be tested. Every name comes from the
 * vocabulary; a type the model proposed is shown by the name it
 * proposed and marked as a proposal.
 */

/** "Carta documento · Vista previa", or "Receta (propuesto)". */
export function typeLabel(
  docType: string,
  stage: string | null,
  proposed: boolean,
  proposedName: string | null = null,
  words: Vocabulary = vocabulary(),
): string {
  const name = documentTypeName(docType, proposedName, words) + (proposed ? " (propuesto)" : "")
  return stage ? `${name} · ${stageName(stage, words)}` : name
}

/** The type of a row of Documentos, or null when it has none yet. */
export function rowTypeLabel(row: DocumentRow, words: Vocabulary = vocabulary()): string | null {
  if (!row.docType) return null
  return typeLabel(row.docType, row.docStage, row.docTypeProposed, row.docTypeName, words)
}

/** Where the type came from, in words. */
export function typeSourceText(found: DocumentType): string {
  switch (found.source) {
    case "marks":
      return "Lo dicen marcas comprobadas en el documento, sin el modelo."
    case "model":
      return "Lo propuso el modelo al leer este documento."
    case "family":
      return "Lo propuso el modelo al leer el documento principal de su familia."
  }
}

/** "Seguridad del modelo: 90 %", when the model said how sure it was. */
export function confidenceText(found: DocumentType): string | null {
  if (found.confidence === null || found.source === "marks") return null
  return `Seguridad del modelo: ${Math.floor(Math.max(0, Math.min(1, found.confidence)) * 100)} %`
}

/** The data of a document grouped by field, in the order they came, with their names. */
export function dataRows(data: Datum[], words: Vocabulary = vocabulary()): { field: string; name: string; values: string[]; byModel: boolean }[] {
  const out: { field: string; name: string; values: string[]; byModel: boolean }[] = []
  for (const datum of data) {
    let row = out.find((r) => r.field === datum.field)
    if (!row) {
      row = { field: datum.field, name: dataFieldName(datum.field, words), values: [], byModel: false }
      out.push(row)
    }
    row.values.push(datum.value)
    row.byModel ||= datum.source === "model"
  }
  return out
}

/** What the digital signatures of a file say, in the vocabulary's words; null without them. */
export function sealText(seal: SealVerdict | null | undefined, words: Vocabulary = vocabulary()): string | null {
  if (!seal) return null
  const time = seal.signedAt ? new Date(seal.signedAt).getTime() : NaN
  const date = Number.isNaN(time) ? null : formatDate(time)
  const template = (date ? words.seal.states[seal.state] : words.seal.states[`${seal.state}-undated`]) ?? words.seal.states[seal.state]
  if (!template) return null
  const parts = [template.replace("{service}", seal.serviceName ?? seal.service ?? "").replace("{date}", date ?? "")]
  const intact = seal.state === "sealed" || seal.state === "valid-unknown"
  if (intact && seal.laterValidation) parts.push(words.seal.validation)
  if (intact && (seal.joined ?? 0) > 0) parts.push(words.seal.joined.replace("{count}", String(seal.joined)))
  if (intact && seal.failing > 0) parts.push(words.seal.failing.replace("{count}", String(seal.failing)))
  return parts.join(" ")
}

/** How many questions a run would ask, in a sentence; null when none. */
export function questionsText(plan: TypesPlan): string | null {
  const total = plan.typeQuestions + plan.dataQuestions
  if (total === 0) return null
  const parts: string[] = []
  if (plan.typeQuestions > 0) parts.push(plural(plan.typeQuestions, "pregunta sobre tipo y etapa", "preguntas sobre tipo y etapa"))
  if (plan.dataQuestions > 0) parts.push(plural(plan.dataQuestions, "pregunta sobre datos", "preguntas sobre datos"))
  return parts.join(" y ")
}

/** The message of the notice before a run, which says what it costs. */
export function runConfirmMessage(plan: TypesPlan, limit: number): string {
  const questions = questionsText(plan) ?? "ninguna pregunta"
  const total = plan.typeQuestions + plan.dataQuestions
  return (
    `Se van a hacer hasta ${questions} al modelo configurado` +
    (total > limit ? ` (en esta vuelta, como mucho ${limit}; el resto queda para después)` : "") +
    ". Cada una consume cupo o costo de ese proveedor. " +
    "Se pregunta una vez por familia de documentos parecidos, sobre su documento principal, y una vez por documento suelto; " +
    "lo que ya se preguntó no se vuelve a preguntar. Puede aparecer alguna pregunta más sobre datos cuando se conozcan los tipos pendientes."
  )
}

/** What a run did, in a sentence. */
export function runReportText(report: TypesRunReport): string {
  const asked = report.asked + report.dataAsked
  const parts = [
    asked === 0 ? "No hizo falta preguntarle nada al modelo." : `Se hicieron ${plural(asked, "pregunta", "preguntas")} al modelo.`,
  ]
  if (report.typed > 0) parts.push(`${plural(report.typed, "documento quedó", "documentos quedaron")} con tipo.`)
  if (report.corrected > 0) parts.push(`${plural(report.corrected, "relación cambió", "relaciones cambiaron")} por las reglas del tipo.`)
  const failed = report.failed + report.dataFailed
  if (failed > 0) parts.push(`${plural(failed, "pregunta falló", "preguntas fallaron")}: ${report.errors[0] ?? ""}`)
  if (report.left > 0) parts.push(`Quedan ${plural(report.left, "pregunta", "preguntas")} para otra vuelta.`)
  return parts.join(" ")
}
