import type { DocumentState, Health, OriginSummary } from "@/commands/docbase"
import type { Tone } from "./page"
import { plural } from "./format"

/**
 * One thing that needs the user's attention. The Inicio screen shows a
 * single list of them, whoever raises each one: the document base and
 * the wiki today; deadlines, tasks and approvals later.
 */
export interface AttentionItem {
  id: string
  tone: Tone
  title: string
  detail: string
  /** Where the user goes to deal with it. */
  target:
    | { view: "origins" }
    | { view: "review" }
    | { view: "documents"; state: DocumentState }
}

const TONE_ORDER: Tone[] = ["error", "warning", "busy", "neutral", "ok"]

export function buildAttention(
  health: Health,
  origins: OriginSummary[],
  reviewPending: number,
): AttentionItem[] {
  const items: AttentionItem[] = []
  const count = (state: DocumentState) => health.byState[state] ?? 0

  for (const origin of origins) {
    if (origin.reachable === false) {
      items.push({
        id: `origin:${origin.id}`,
        tone: "warning",
        title: `No se puede leer "${origin.name}"`,
        detail: "La carpeta no está o aparece vacía. Se conserva lo que se sabía de sus archivos hasta que vuelva.",
        target: { view: "origins" },
      })
    }
  }
  if (count("unreadable") > 0) {
    items.push({
      id: "unreadable",
      tone: "error",
      title: plural(count("unreadable"), "archivo no se pudo leer", "archivos no se pudieron leer"),
      detail: "Pueden estar abiertos en otro programa o sin descargar. Se vuelve a intentar en cada barrido.",
      target: { view: "documents", state: "unreadable" },
    })
  }
  if (count("error") > 0) {
    items.push({
      id: "error",
      tone: "error",
      title: plural(count("error"), "documento falló al obtener su texto", "documentos fallaron al obtener su texto"),
      detail: "El archivo se leyó, pero no se pudo extraer su texto. El detalle de cada uno dice por qué.",
      target: { view: "documents", state: "error" },
    })
  }
  if (count("needs-recognition") > 0) {
    items.push({
      id: "needs-recognition",
      tone: "warning",
      title: plural(count("needs-recognition"), "documento tiene el texto en imagen", "documentos tienen el texto en imagen"),
      detail: "Son escaneos o fotos. Su texto se reconoce cuando se incorporan al wiki, una sola vez por documento.",
      target: { view: "documents", state: "needs-recognition" },
    })
  }
  if (count("too-large") > 0) {
    items.push({
      id: "too-large",
      tone: "warning",
      title: plural(count("too-large"), "archivo es demasiado grande", "archivos son demasiado grandes"),
      detail: "Quedan en el inventario, sin leer ni comparar con otros.",
      target: { view: "documents", state: "too-large" },
    })
  }
  if (count("link") > 0) {
    items.push({
      id: "link",
      tone: "neutral",
      title: plural(count("link"), "documento de un disco en la nube quedó afuera", "documentos de un disco en la nube quedaron afuera"),
      detail: "En la carpeta solo hay un acceso directo, sin contenido: se detectan, pero no se pueden procesar.",
      target: { view: "documents", state: "link" },
    })
  }
  if (reviewPending > 0) {
    items.push({
      id: "review",
      tone: "busy",
      title: plural(reviewPending, "propuesta del wiki espera tu revisión", "propuestas del wiki esperan tu revisión"),
      detail: "Contradicciones, páginas duplicadas o faltantes que el wiki encontró al incorporar documentos.",
      target: { view: "review" },
    })
  }
  return items.sort((a, b) => TONE_ORDER.indexOf(a.tone) - TONE_ORDER.indexOf(b.tone))
}

/** Share of the processable files whose text is already in the base, 0 to 100. */
export function processedPercent(health: Health): number | null {
  const count = (state: DocumentState) => health.byState[state] ?? 0
  const processable = health.locations - count("link") - count("unsupported") - count("excluded")
  if (processable <= 0) return null
  return Math.round((count("processed") / processable) * 1000) / 10
}

/** Files the base has not finished with: waiting to be read or to get their text. */
export function pendingCount(health: Health): number {
  return (health.byState.unread ?? 0) + (health.byState.unprocessed ?? 0)
}
