import type { ConfidenceEffect, QualityDocument } from "@/commands/docbase"
import { formatCount, plural } from "./format"

/**
 * What «Calidad de la lectura» and the screen of the thresholds say, apart
 * from the screens so it can be tested (P-542, P-544). A share of safe
 * words is the firm words over every word read; a document below «no
 * legible» or below the type of low confidence is of low confidence: it is
 * only for queries.
 */

/** «37 %». */
export function percent(share: number): string {
  return `${formatCount(Math.round(share * 100))} %`
}

/** Where the document is in its file and how many pages it has. */
export function pagesText(document: QualityDocument): string {
  if (document.firstPage !== null && document.lastPage !== null) {
    return document.firstPage === document.lastPage
      ? `pág. ${document.firstPage} del archivo`
      : `págs. ${document.firstPage} a ${document.lastPage} del archivo`
  }
  return document.pages !== null ? plural(document.pages, "página", "páginas") : "páginas sin contar"
}

/** How many pages a document counts for, as the base counts them. */
export function pagesOf(document: QualityDocument): number {
  return document.pages ?? 1
}

/** How many documents and pages have a share below `value` (the same rule as the base). */
export function below(list: readonly QualityDocument[], value: number): { documents: number; pages: number } {
  const under = list.filter((document) => document.share < value)
  return { documents: under.length, pages: under.reduce((sum, document) => sum + pagesOf(document), 0) }
}

/** The verdict with the thresholds of now, in words. */
export function verdictText(verdict: QualityDocument["verdict"]): string {
  switch (verdict) {
    case "no-legible":
      return "No legible"
    case "low-confidence":
      return "Tipo de confianza baja"
    default:
      return "Se usa"
  }
}

/** The words of each band of a document: «12 seguras, 30 aproximadas y 4 descartadas». */
export function wordsText(document: QualityDocument): string {
  return `${formatCount(document.firm)} seguras, ${formatCount(document.approximate)} aproximadas y ${formatCount(document.discarded)} descartadas`
}

/** What keeping the two thresholds of the confidence would do, before it is kept. */
export function effectText(effect: ConfidenceEffect): string {
  if (effect.toLow === 0 && effect.fromLow === 0) {
    return "Con estos valores ningún documento cambia de lado: no se pregunta nada."
  }
  const parts: string[] = []
  if (effect.toLow > 0) {
    parts.push(
      `${plural(effect.toLow, "documento pasa", "documentos pasan")} a confianza baja (${plural(effect.toLowPages, "página", "páginas")}): lo que dieron sale de la wiki en la próxima resolución, sin preguntar nada.`,
    )
  }
  if (effect.fromLow > 0) {
    parts.push(
      `${plural(effect.fromLow, "documento deja", "documentos dejan")} de ser de confianza baja (${plural(effect.fromLowPages, "página", "páginas")}): su lectura ya está guardada y no se vuelve a pagar; vuelven a dar datos.`,
    )
  }
  parts.push(
    effect.pagesWrittenAgain > 0
      ? `Las páginas de personas, organizaciones y casos cuyos datos cambien se vuelven a escribir: a lo sumo ${plural(effect.pagesWrittenAgain, "página", "páginas")}, una consulta cada una a «Preguntar al modelo», con la lectura automática encendida.`
      : "Ninguna página de personas, organizaciones o casos cambia: no se pregunta nada.",
  )
  return parts.join(" ")
}
